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
        // + git clone https://github.com/anchao/nuttx --branch 25020501 nuttx
        // + git clone https://github.com/anchao/nuttx-apps --branch 25020501 apps
        // NuttX Source: https://github.com/apache/nuttx/tree/fa059c19fad275324afdfec023d24a85827516e9
        // NuttX Apps: https://github.com/apache/nuttx-apps/tree/6d0afa6c9b8d4ecb896f9aa177dbdfcd40218f48
        // + tools/configure.sh rv-virt:knsh64
        // + spawn qemu-system-riscv64 -semihosting -M virt,aclint=on -cpu rv64 -kernel nuttx -nographic
        // + qemu-system-riscv64 --version
        // QEMU emulator version 9.2.0
        // OpenSBI v1.5.1
        // nsh> uname -a
        // NuttX 10.4.0 fa059c19fa Feb  5 2025 19:25:45 risc-v rv-virt
        // nsh> ostest
        // ostest_main: Exiting with status 0
        let log_content = log_content.replace("\n\n", "\n");
        let log_index = log_content.len();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let log_index = log_content[0..log_index].rfind('\n').unwrap();
        let mut result = 
            if status.success() { format!("Build and Test Successful (rv-virt:{script})\n") }
            else { format!("Build and Test **FAILED** (rv-virt:{script})\n") };
        result.push_str(&snippet_url);
        result.push_str("\n```text");
        result.push_str(&log_content[log_index..]);
        result.push_str("```\n");
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
    issues.create_comment(pr_id, comment_text).await?;

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
