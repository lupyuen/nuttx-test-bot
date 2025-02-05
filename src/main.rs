//! Fetch the Latest 20 Unread Notifications:
//!   If Mention = "@nuttxpr test rv-virt:knsh64"
//!   - Build and Test NuttX
//!   - Capture the Output Log
//!   - Extract the Log Output and Result
//!   - Post as PR Comment
//!   - Post to Mastodon
//!   - Allow only Specific People

use std::{
    env, 
    thread::sleep, 
    time::Duration
};
use clap::Parser;
use log::info;
use octocrab::{
    issues::IssueHandler, 
    models::{reactions::ReactionContent, IssueState, Label}, 
    params,
    pulls::PullRequestHandler
};

/// Command-Line Arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Owner of the GitHub Repo that will be processed (`apache`)
    #[arg(long)]
    owner: String,

    /// Name of the GitHub Repo that will be processed (`nuttx` or `nuttx-apps`)
    #[arg(long)]
    repo: String,
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
        .personal_token(token)
        .build()?;

    let notifications = octocrab
        .activity()
        .notifications()
        .list()
        .all(true)
        .send()
        .await?;
    for n in notifications {
        let title = &n.subject.title;
        let reason = &n.reason;
        let url = &n.url;
        let latest_comment_url = &n.subject.latest_comment_url.unwrap();
        println!("title={title}", );
        println!("reason={reason}", );
        println!("url={url}", );
        println!("latest_comment_url={latest_comment_url}", );
        // println!("n={n:?}");
        break;
    }

    // // Get the Handlers for GitHub Pull Requests and Issues
    // let pulls = octocrab.pulls(&args.owner, &args.repo);
    // let issues = octocrab.issues(&args.owner, &args.repo);

    // // Fetch the 20 Newest Pull Requests that are Open
    // let pr_list = pulls
    //     .list()
    //     .state(params::State::Open)
    //     .sort(params::pulls::Sort::Created)
    //     .direction(params::Direction::Descending)
    //     .per_page(20)
    //     .send()
    //     .await?;

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
 */

/// Validate the PR. Then post the results as a PR Comment
async fn process_pr(pulls: &PullRequestHandler<'_>, issues: &IssueHandler<'_>, pr_id: u64) -> Result<(), Box<dyn std::error::Error>> {
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

    // Skip if PR contains Comments
    if pr.comments.unwrap() > 0 {
        info!("Skipping PR with comments: {}", pr_id);
        return Ok(());
    }

    // Skip if PR Size is Unknown
    let labels = pr.labels.unwrap();
    if labels.is_empty() {
        info!("Skipping Unknown PR Size: {}", pr_id);
        return Ok(());
    }

    // Skip if PR Size is XS
    let size_xs: Vec<Label> = labels
        .into_iter()
        .filter(|l| l.name == "Size: XS")
        .collect();
    if size_xs.len() > 0 {
        info!("Skipping PR Size XS: {}", pr_id);
        return Ok(());
    }

    // Fetch the PR Commits
    // TODO: Change `pull_number` to `pr_commits`
    let commits = pulls
        .pull_number(pr_id)
        .commits()
        .await;
    let commits = commits.unwrap().items;
    let mut precheck = String::new();

    // Check for Multiple Commits
    // if commits.len() > 1 {
    //     precheck.push_str(
    //         &format!("__Squash The Commits:__ This PR contains {} Commits. Please Squash the Multiple Commits into a Single Commit.\n\n", commits.len())
    //     );
    // }

    // Check for Empty Commit Message
    let mut empty_message = false;
    for commit in commits.iter() {
        // Message should be "title\n\nbody"
        let message = &commit.commit.message;
        if message.find("\n").is_some() {
        } else {
            info!("Missing Commit Message: {:#?}", message);
            empty_message = true;
            break;
        }
    }
    if empty_message {
        precheck.push_str(
            "__Fill In The Commit Message:__ This PR contains a Commit with an Empty Commit Message. Please fill in the Commit Message with the PR Summary.\n\n"
        );
    }

    // Get the PR Body
    let body = pr.body.unwrap_or("".to_string());
    info!("PR Body: {:#?}", body);

    // Retry Gemini API up to 3 times, by checking the PR Reactions.
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
    let response_text = "";

    // Compose the PR Comment
    let comment_text =
        header.to_string() + "\n\n" +
        &precheck + "\n\n" +
        &response_text;

    // Post the PR Comment
    let comment = issues
        .create_comment(pr_id, comment_text)
        .await?;
    info!("PR Comment: {:#?}", comment);       

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
