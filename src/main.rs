//! Fetch the Latest 20 Unread Notifications:
//!   If Mention = "@nuttxpr test rv-virt:knsh64"
//!   - Build and Test NuttX
//!   - Capture the Output Log
//!   - Extract the Log Output and Result
//!   - Post as PR Comment
//!   - Post to Mastodon
//!   - Allow only Specific People

use std::{
    fs, process::Command, thread::sleep, time::Duration
};
use clap::Parser;
use log::info;
use octocrab::{
    issues::IssueHandler, 
    models::{reactions::ReactionContent, IssueState}, 
    pulls::PullRequestHandler
};
use serde_json::Value;

/// Command-Line Arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
}

/// Validate the Latest PRs and post the PR Reviews as PR Comments
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init the Logger and Command-Line Args
    env_logger::init();
    let args = Args::parse();

    // Init the GitHub Client
    let token = std::env::var("GITHUB_TOKEN")
        .expect("GITHUB_TOKEN env variable is required");
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(token.clone())
        .build()?;

    // Fetch all Notifications
    let notifications = octocrab
        .activity()
        .notifications()
        .list()
        .all(true)
        .send()
        .await?;

    // For Every Notification...
    for n in notifications {
        // Handle only Mentions
        let reason = n.reason;  // "mention"
        println!("reason={reason}", );
        if reason != "mention" { continue; }

        // TODO: Fetch the Mentioned Comment "@nuttxpr test rv-virt:knsh64"
        let owner = n.repository.owner.unwrap().login;
        let repo = n.repository.name;
        let pr_title = n.subject.title;  // "Testing our bot"
        let pr_url = n.subject.url.unwrap();  // https://api.github.com/repos/lupyuen2/wip-nuttx/pulls/88
        let thread_url = n.url;  // https://api.github.com/notifications/threads/14630615157
        let latest_comment_url = &n.subject.latest_comment_url.unwrap();  // https://api.github.com/repos/lupyuen2/wip-nuttx/issues/comments/2635685180
        println!("owner={owner}");
        println!("repo={repo}");
        println!("pr_title={pr_title}");
        println!("pr_url={pr_url}");
        println!("thread_url={thread_url}");
        println!("latest_comment_url={latest_comment_url}");
        // println!("n={n:?}");

        // Fetch the PR
        // pr_url looks like https://api.github.com/repos/lupyuen2/wip-nuttx/pulls/88
        let client = reqwest::Client::new();
        let res = client
            .get(pr_url.clone())
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "nuttx-rewind-notify")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?;
        // println!("res={res:?}");
        if !res.status().is_success() {
            println!("*** Get PR Failed: {pr_url}");
            sleep(Duration::from_secs(30));
            continue;
        }
        // println!("Status: {}", res.status());
        // println!("Headers:\n{:#?}", res.headers());
        let body = res.text().await?;
        // println!("Body: {body}");

        // Get the Head Ref and Head URL from PR
        // TODO: Get PR Owner and Repo
        let pr: Value = serde_json::from_str(&body).unwrap();
        let pr_id = pr["number"].as_u64().unwrap();
        let head = &pr["head"];
        let head_ref = head["ref"].as_str().unwrap();  // "test-bot"
        let head_url = head["repo"]["html_url"].as_str().unwrap();  // https://github.com/lupyuen2/wip-nuttx
        println!("pr_id={pr_id}");
        println!("head_ref={head_ref}");
        println!("head_url={head_url}");

        // True if URL is an Apps Repo
        let is_apps =
            if head_url.contains("apps") { true }
            else { false };

        // Set the URLs and Refs for NuttX and Apps
        let nuttx_hash = "HEAD";
        let nuttx_url =
            if is_apps { "https://github.com/apache/nuttx" }
            else { head_url };
        let nuttx_ref =
            if is_apps { "master" }
            else { head_ref };
        let apps_hash = "HEAD";
        let apps_url = 
            if is_apps { head_url }
            else { "https://github.com/apache/nuttx-apps" };
        let apps_ref =
            if is_apps { head_ref }
            else { "master" };

        // Build and Test NuttX: ./build-test.sh knsh64 /tmp/build-test.log HEAD HEAD https://github.com/apache/nuttx master https://github.com/apache/nuttx-apps master
        // Which calls: ./build-test-knsh64.sh HEAD HEAD https://github.com/apache/nuttx master https://github.com/apache/nuttx-apps master
        let cmd = format!("./build-test-knsh64.sh {nuttx_hash} {apps_hash} {nuttx_url} {nuttx_ref} {apps_url} {apps_ref}");
        println!("cmd={cmd}");
        let script = "knsh64";
        let log = "/tmp/build-test.log";
        let mut child = Command
            ::new("../nuttx-build-farm/build-test.sh")
            .arg(script).arg(log)
            .arg(nuttx_hash).arg(apps_hash)
            .arg(nuttx_url).arg(nuttx_ref)
            .arg(apps_url).arg(apps_ref)
            .spawn().unwrap();
        // println!("child={child:?}");

        // Wait for Build and Test to complete
        let status = child.wait().unwrap();  // 0 if successful
        println!("status={status:?}");

        // Upload the log as GitLab Snippet
        let log_content = fs::read_to_string(log).unwrap();
        let snippet_url = create_snippet(&log_content).await?;

        // TODO: Extract the Result and Log Output
        let mut result = 
            if status.success() { format!("Build and Test Successful (rv-virt:{script})\n") }
            else { format!("Build and Test **FAILED** (rv-virt:{script})\n") };
        result.push_str(&snippet_url);
        println!("result={result}");

        // Get the Handlers for GitHub Pull Requests and Issues
        let pulls = octocrab.pulls(&owner, &repo);
        let issues = octocrab.issues(&owner, &repo);

        // Post the Result and Log Output as PR Comment
        process_pr(&pulls, &issues, pr_id, &result).await?;

        // TODO: Post to Mastodon
        // TODO: Allow only Specific People
        break;
    }

    // // Every 5 Seconds: Process the next PR fetched
    // for pr in pr_list {
    //     let pr_id = pr.number;
    //     process_pr(&pulls, &issues, pr_id)
    //         .await?;
    //     sleep(Duration::from_secs(5));
    // }

    // Return OK
    Ok(())
}

/*
Notification { id: NotificationId(14630615157), repository: Repository { id: RepositoryId(566669181), node_id: Some("R_kgDOIcavfQ"), name: "wip-nuttx", full_name: Some("lupyuen2/wip-nuttx"), owner: Some(Author { login: "lupyuen2", id: UserId(88765682), node_id: "MDEyOk9yZ2FuaXphdGlvbjg4NzY1Njgy", avatar_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("avatars.githubusercontent.com")), port: None, path: "/u/88765682", query: Some("v=4"), fragment: None
            }, gravatar_id: "", url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2", query: None, fragment: None
            }, html_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("github.com")), port: None, path: "/lupyuen2", query: None, fragment: None
            }, followers_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/followers", query: None, fragment: None
            }, following_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/following%7B/other_user%7D", query: None, fragment: None
            }, gists_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/gists%7B/gist_id%7D", query: None, fragment: None
            }, starred_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/starred%7B/owner%7D%7B/repo%7D", query: None, fragment: None
            }, subscriptions_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/subscriptions", query: None, fragment: None
            }, organizations_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/orgs", query: None, fragment: None
            }, repos_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/repos", query: None, fragment: None
            }, events_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/events%7B/privacy%7D", query: None, fragment: None
            }, received_events_url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/users/lupyuen2/received_events", query: None, fragment: None
            }, type: "Organization", site_admin: false, patch_url: None, email: None
        }), private: Some(false), html_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("github.com")), port: None, path: "/lupyuen2/wip-nuttx", query: None, fragment: None
        }), description: Some("(Work-in-Progress for SG2000, Ox64, Star64 and PinePhone) Apache NuttX is a mature, real-time embedded operating system (RTOS)"), fork: Some(true), url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx", query: None, fragment: None
        }, archive_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/%7Barchive_format%7D%7B/ref%7D", query: None, fragment: None
        }), assignees_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/assignees%7B/user%7D", query: None, fragment: None
        }), blobs_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/git/blobs%7B/sha%7D", query: None, fragment: None
        }), branches_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/branches%7B/branch%7D", query: None, fragment: None
        }), collaborators_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/collaborators%7B/collaborator%7D", query: None, fragment: None
        }), comments_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/comments%7B/number%7D", query: None, fragment: None
        }), commits_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/commits%7B/sha%7D", query: None, fragment: None
        }), compare_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/compare/%7Bbase%7D...%7Bhead%7D", query: None, fragment: None
        }), contents_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/contents/%7B+path%7D", query: None, fragment: None
        }), contributors_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/contributors", query: None, fragment: None
        }), deployments_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/deployments", query: None, fragment: None
        }), downloads_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/downloads", query: None, fragment: None
        }), events_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/events", query: None, fragment: None
        }), forks_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/forks", query: None, fragment: None
        }), git_commits_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/git/commits%7B/sha%7D", query: None, fragment: None
        }), git_refs_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/git/refs%7B/sha%7D", query: None, fragment: None
        }), git_tags_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/git/tags%7B/sha%7D", query: None, fragment: None
        }), git_url: None, issue_comment_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/issues/comments%7B/number%7D", query: None, fragment: None
        }), issue_events_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/issues/events%7B/number%7D", query: None, fragment: None
        }), issues_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/issues%7B/number%7D", query: None, fragment: None
        }), keys_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/keys%7B/key_id%7D", query: None, fragment: None
        }), labels_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/labels%7B/name%7D", query: None, fragment: None
        }), languages_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/languages", query: None, fragment: None
        }), merges_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/merges", query: None, fragment: None
        }), milestones_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/milestones%7B/number%7D", query: None, fragment: None
        }), notifications_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/notifications%7B", query: Some("since,all,participating}"), fragment: None
        }), pulls_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/pulls%7B/number%7D", query: None, fragment: None
        }), releases_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/releases%7B/id%7D", query: None, fragment: None
        }), ssh_url: None, stargazers_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/stargazers", query: None, fragment: None
        }), statuses_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/statuses/%7Bsha%7D", query: None, fragment: None
        }), subscribers_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/subscribers", query: None, fragment: None
        }), subscription_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/subscription", query: None, fragment: None
        }), tags_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/tags", query: None, fragment: None
        }), teams_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/teams", query: None, fragment: None
        }), trees_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/git/trees%7B/sha%7D", query: None, fragment: None
        }), clone_url: None, mirror_url: None, hooks_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/hooks", query: None, fragment: None
        }), svn_url: None, homepage: None, language: None, forks_count: None, stargazers_count: None, watchers_count: None, size: None, default_branch: None, open_issues_count: None, is_template: None, topics: None, has_issues: None, has_projects: None, has_wiki: None, has_pages: None, has_downloads: None, archived: None, disabled: None, visibility: None, pushed_at: None, created_at: None, updated_at: None, permissions: None, allow_rebase_merge: None, template_repository: None, allow_squash_merge: None, allow_merge_commit: None, allow_update_branch: None, allow_forking: None, subscribers_count: None, network_count: None, license: None, allow_auto_merge: None, delete_branch_on_merge: None, parent: None, source: None
    }, subject: Subject { title: "Testing our bot", url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/pulls/88", query: None, fragment: None
        }), latest_comment_url: Some(Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/repos/lupyuen2/wip-nuttx/issues/comments/2635666191", query: None, fragment: None
        }), type: "PullRequest"
    }, reason: "mention", unread: true, updated_at: 2025-02-05T04: 17: 47Z, last_read_at: None, url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None, host: Some(Domain("api.github.com")), port: None, path: "/notifications/threads/14630615157", query: None, fragment: None
    }
}

Thread:
{
  "id": "14630615157",
  "unread": true,
  "reason": "mention",
  "updated_at": "2025-02-05T04:17:47Z",
  "last_read_at": null,
  "subject": {
    "title": "Testing our bot",
    "url": "https://api.github.com/repos/lupyuen2/wip-nuttx/pulls/88",
    "latest_comment_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/issues/comments/2635666191",
    "type": "PullRequest"
  },
  "repository": {
    "id": 566669181,
    "node_id": "R_kgDOIcavfQ",
    "name": "wip-nuttx",
    "full_name": "lupyuen2/wip-nuttx",
    "private": false,
    "owner": {
      "login": "lupyuen2",
      "id": 88765682,
      "node_id": "MDEyOk9yZ2FuaXphdGlvbjg4NzY1Njgy",
      "avatar_url": "https://avatars.githubusercontent.com/u/88765682?v=4",
      "gravatar_id": "",
      "url": "https://api.github.com/users/lupyuen2",
      "html_url": "https://github.com/lupyuen2",
      "followers_url": "https://api.github.com/users/lupyuen2/followers",
      "following_url": "https://api.github.com/users/lupyuen2/following{/other_user}",
      "gists_url": "https://api.github.com/users/lupyuen2/gists{/gist_id}",
      "starred_url": "https://api.github.com/users/lupyuen2/starred{/owner}{/repo}",
      "subscriptions_url": "https://api.github.com/users/lupyuen2/subscriptions",
      "organizations_url": "https://api.github.com/users/lupyuen2/orgs",
      "repos_url": "https://api.github.com/users/lupyuen2/repos",
      "events_url": "https://api.github.com/users/lupyuen2/events{/privacy}",
      "received_events_url": "https://api.github.com/users/lupyuen2/received_events",
      "type": "Organization",
      "user_view_type": "public",
      "site_admin": false
    },
    "html_url": "https://github.com/lupyuen2/wip-nuttx",
    "description": "(Work-in-Progress for SG2000, Ox64, Star64 and PinePhone) Apache NuttX is a mature, real-time embedded operating system (RTOS)",
    "fork": true,
    "url": "https://api.github.com/repos/lupyuen2/wip-nuttx",
    "forks_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/forks",
    "keys_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/keys{/key_id}",
    "collaborators_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/collaborators{/collaborator}",
    "teams_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/teams",
    "hooks_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/hooks",
    "issue_events_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/issues/events{/number}",
    "events_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/events",
    "assignees_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/assignees{/user}",
    "branches_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/branches{/branch}",
    "tags_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/tags",
    "blobs_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/git/blobs{/sha}",
    "git_tags_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/git/tags{/sha}",
    "git_refs_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/git/refs{/sha}",
    "trees_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/git/trees{/sha}",
    "statuses_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/statuses/{sha}",
    "languages_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/languages",
    "stargazers_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/stargazers",
    "contributors_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/contributors",
    "subscribers_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/subscribers",
    "subscription_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/subscription",
    "commits_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/commits{/sha}",
    "git_commits_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/git/commits{/sha}",
    "comments_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/comments{/number}",
    "issue_comment_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/issues/comments{/number}",
    "contents_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/contents/{+path}",
    "compare_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/compare/{base}...{head}",
    "merges_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/merges",
    "archive_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/{archive_format}{/ref}",
    "downloads_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/downloads",
    "issues_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/issues{/number}",
    "pulls_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/pulls{/number}",
    "milestones_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/milestones{/number}",
    "notifications_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/notifications{?since,all,participating}",
    "labels_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/labels{/name}",
    "releases_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/releases{/id}",
    "deployments_url": "https://api.github.com/repos/lupyuen2/wip-nuttx/deployments"
  },
  "url": "https://api.github.com/notifications/threads/14630615157",
  "subscription_url": "https://api.github.com/notifications/threads/14630615157/subscription"
}
 */

/// Validate the PR. Then post the results as a PR Comment
async fn process_pr(pulls: &PullRequestHandler<'_>, issues: &IssueHandler<'_>, pr_id: u64, response_text: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Fetch the PR
    let pr = pulls
        .get(pr_id)
        .await?;
    info!("{:#?}", pr.url);

    // Skip if PR State is Not Open
    if pr.state.unwrap() != IssueState::Open {
        info!("Skipping Closed PR: {}", pr_id);
        return Ok(());
    }

    // Fetch the PR Reactions. Quit if Both Reactions are set.
    let reactions = get_reactions(issues, pr_id).await?;
    if reactions.0.is_some() && reactions.1.is_some() {
        info!("Skipping PR after 3 retries: {}", pr_id);
        return Ok(());
    }

    // Bump up the PR Reactions: 00 > 01 > 10 > 11
    bump_reactions(issues, pr_id, reactions).await?;

    // Header for PR Comment
    let header = "[**\\[Experimental Bot, please feedback here\\]**](https://github.com/search?q=repo%3Aapache%2Fnuttx+13552&type=issues)";

    // Compose the PR Comment
    let comment_text =
        header.to_string() + "\n\n" +
        response_text;

    // Post the PR Comment
    let comment = issues
        .create_comment(pr_id, comment_text)
        .await?;
    // info!("PR Comment: {:#?}", comment);       

    // If successful, delete the PR Reactions
    delete_reactions(issues, pr_id).await?;
    info!("{:#?}", pr.url);

    // Wait 1 minute
    sleep(Duration::from_secs(60));

    // Return OK
    Ok(())
}

/// Return the Reaction IDs for Rocket and Eyes Reactions, created by the Bot
async fn get_reactions(issues: &IssueHandler<'_>, pr_id: u64) -> 
    Result<(Option<u64>, Option<u64>), Box<dyn std::error::Error>> {
    // Fetch the PR Reactions
    let reactions = issues
        .list_reactions(pr_id)
        .send()
        .await?;
    let reactions = reactions.items;

    // Watch for Rocket and Eyes Reactions created by the Bot
    // TODO: Change `nuttxpr` to the GitHub User ID of the Bot
    let mut result: (Option<u64>, Option<u64>) = (None, None);
    for reaction in reactions.iter() {
        let content = &reaction.content;
        let user = &reaction.user.login;
        let reaction_id = &reaction.id.0;
        if user == "nuttxpr" {
            match content {
                ReactionContent::Rocket => { result.0 = Some(*reaction_id) }
                ReactionContent::Eyes   => { result.1 = Some(*reaction_id) }
                _ => {}
            }
        }
    }
    Ok(result)
}

/// Bump up the 2 PR Reactions: 00 > 01 > 10 > 11
/// Position 0 is the Rocket Reaction, Position 1 is the Eye Reaction
async fn bump_reactions(issues: &IssueHandler<'_>, pr_id: u64, reactions: (Option<u64>, Option<u64>)) -> 
    Result<(), Box<dyn std::error::Error>> {
    match reactions {
        // (Rocket, Eye)
        (None,     None)    => { create_reaction(issues, pr_id, ReactionContent::Rocket).await?; }
        (Some(id), None)    => { delete_reaction(issues, pr_id, id).await?; create_reaction(issues, pr_id, ReactionContent::Eyes).await?; }
        (None,     Some(_)) => { create_reaction(issues, pr_id, ReactionContent::Rocket).await?; }
        (Some(_),  Some(_)) => { panic!("Reaction Overflow") }
    }
    Ok(())
}

/// Delete the PR Reactions
async fn delete_reactions(issues: &IssueHandler<'_>, pr_id: u64) -> 
    Result<(), Box<dyn std::error::Error>> {
    let reactions = get_reactions(issues, pr_id).await?;
    if let Some(reaction_id) = reactions.0 {
        delete_reaction(issues, pr_id, reaction_id).await?;
    }
    if let Some(reaction_id) = reactions.1 {
        delete_reaction(issues, pr_id, reaction_id).await?;
    }
    Ok(())
}

/// Create the PR Reaction
async fn create_reaction(issues: &IssueHandler<'_>, pr_id: u64, content: ReactionContent) -> 
    Result<(), Box<dyn std::error::Error>> {
    issues.create_reaction(pr_id, content)
        .await?;
    Ok(())
}

/// Delete the PR Reaction
async fn delete_reaction(issues: &IssueHandler<'_>, pr_id: u64, reaction_id: u64) -> 
    Result<(), Box<dyn std::error::Error>> {
    issues.delete_reaction(pr_id, reaction_id)
        .await?;
    Ok(())
}

/// Create a GitLab Snippet. Returns the Snippet URL.
/// https://docs.gitlab.com/ee/api/snippets.html#create-new-snippet
async fn create_snippet(content: &str) -> Result<String, Box<dyn std::error::Error>> {
    let user = "lupyuen";
    let repo = "nuttx-build-log";
    let body = r#"
{
  "title": "NuttX Test Bot",
  "description": "Build-Test Log",
  "visibility": "public",
  "files": [
    {
      "content": "Hello world",
      "file_path": "nuttx-test-bot.log"
    }
  ]
}
    "#;
    let mut body: Value = serde_json::from_str(&body).unwrap();
    body["files"][0]["content"] = content.into();

    let token = std::env::var("GITLAB_TOKEN")
        .expect("GITLAB_TOKEN env variable is required");
    let client = reqwest::Client::new();
    let gitlab = format!("https://gitlab.com/api/v4/projects/{user}%2F{repo}/snippets");
    let res = client
        .post(gitlab)
        .header("Content-Type", "application/json")
        .header("PRIVATE-TOKEN", token)      
        .body(body.to_string())
        .send()
        .await?;
    // println!("res={res:?}");
    if !res.status().is_success() {
        println!("*** Create Snippet Failed: {user} @ {repo}");
        sleep(Duration::from_secs(30));
        panic!();
    }
    // println!("Status: {}", res.status());
    // println!("Headers:\n{:#?}", res.headers());
    let response = res.text().await?;
    // println!("response={response}");
    let response: Value = serde_json::from_str(&response).unwrap();
    let url = response["web_url"].as_str().unwrap();
    Ok(url.into())
}
