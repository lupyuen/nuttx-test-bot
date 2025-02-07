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
use bit_vec::BitVec;
use clap::Parser;
use log::info;
use octocrab::{
    issues::IssueHandler, 
    models::{pulls::PullRequest, reactions::ReactionContent, IssueState}, 
    pulls::PullRequestHandler
};
use regex::Regex;
use serde_json::Value;
use url::Url;

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
    // TODO: Unread only
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
        let reason = &n.reason;  // "mention"
        // println!("reason={reason}", );
        if reason != "mention" { continue; }
        // TODO: Mark Notification as Read

        // Fetch the PR from the Notification
        let owner = n.repository.owner.clone().unwrap().login;
        let repo = n.repository.name.clone();
        let pr_title = &n.subject.title;  // "Testing our bot"
        let pr_url = n.subject.url.clone().unwrap();  // https://api.github.com/repos/lupyuen2/wip-nuttx/pulls/88
        let thread_url = &n.url;  // https://api.github.com/notifications/threads/14630615157
        println!("owner={owner}");
        println!("repo={repo}");
        println!("pr_title={pr_title}");
        println!("pr_url={pr_url}");
        println!("thread_url={thread_url}");
        if !pr_url.as_str().contains("/pulls/") { println!("Not a PR: {pr_url}"); continue; }
        // println!("n={n:#?}");

        // Extract the PR Number
        let regex = Regex::new(".*/([0-9]+)$").unwrap();
        let caps = regex.captures(pr_url.as_str()).unwrap();
        let pr_id_str = caps.get(1).unwrap().as_str();
        let pr_id: u64 = pr_id_str.parse().unwrap();
        println!("pr_id={pr_id}");

        // Allow only Specific Repos: apache/nuttx, apache/nuttx-apps
        if owner != "apache" ||
            !["nuttx", "nuttx-apps"].contains(&repo.as_str()) {
            println!("Disallowed owner/repo: {owner}/{repo}");
            continue;
        }

        // Get the Handlers for GitHub Pull Requests and Issues
        let pulls = octocrab.pulls(&owner, &repo);
        let issues = octocrab.issues(&owner, &repo);

        // Post the Result and Log Output as PR Comment
        process_pr(&pulls, &issues, pr_id).await?;

        // Wait 1 minute
        sleep(Duration::from_secs(60));

        // TODO: Mark Notification as Read
        // TODO: Continue to Next Notification
        break;

        // TODO: Allow only Specific People
        // TODO: Post to Mastodon
    }

    // Return OK
    Ok(())
}

/// Build and Test the PR. Then post the results as a PR Comment
async fn process_pr(pulls: &PullRequestHandler<'_>, issues: &IssueHandler<'_>, pr_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    // Get the Command and Args: ["test", "milkv_duos:nsh"]
    let args = get_command(issues, pr_id).await?;
    if args.is_none() { println!("Missing command"); return Ok(()); }
    let args = args.unwrap();
    let cmd = &args[0];
    let target = &args[1];
    if cmd != "test" { println!("Unknown command: {cmd}"); return Ok(()); }
    let (script, target) = match target.as_str() {
        "milkv_duos:nsh" => ("oz64", target),
        "oz64:nsh"       => ("oz64", &"milkv_duos:nsh".into()),
        "rv-virt:knsh64" => ("knsh64", target),
        _ => { println!("Unknown target: {target}"); return Ok(()); }
    };
    println!("target={target}");
    println!("script={script}");

    // std::process::exit(0); ////
    println!("PLEASE VERIFY");
    sleep(Duration::from_secs(30));

    // Fetch the PR
    let pr = pulls
        .get(pr_id)
        .await?;
    info!("{:#?}", pr.url);

    // Skip if PR State is Not Open
    if pr.state.clone().unwrap() != IssueState::Open {
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

    // Build and Test the PR
    let response_text = build_test(&pr, target, script).await?;

    // Header for PR Comment
    let header = "[**\\[Experimental Bot, please feedback here\\]**](https://github.com/search?q=repo%3Aapache%2Fnuttx+15779&type=issues)";

    // Compose the PR Comment
    let comment_text =
        header.to_string() + "\n\n" +
        &response_text;

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

/// Get the Last Command from the PR: "@nuttxpr test milkv_duos:nsh" becomes ["test", "milkv_duos:nsh"]
async fn get_command(issues: &IssueHandler<'_>, pr_id: u64) -> Result<Option<Vec<String>>, Box<dyn std::error::Error>> {
    let comments = issues
        .list_comments(pr_id)
        .send()
        .await?;
    for comment in comments {
        let user = &comment.user.login;  // "nuttxpr"
        let body = &comment.body.clone().unwrap_or("".into());
        let body = body.trim().replace("  ", " ");  // "@nuttxpr test milkv_duos:nsh"

        // if user == "nuttxpr" { println!("Skipping, already handled"); break; }
        if !body.starts_with("@nuttxpr") { continue; }
        println!("body={body}");

        // body contains "@nuttxpr test milkv_duos:nsh"
        // Parse the command
        let mut args: Vec<String> = body.split_whitespace().map(|v| v.to_string()).collect();
        args.remove(0);  // ["test", "milkv_duos:nsh"]
        println!("args={args:?}");
        return Ok(Some(args));
    }
    Ok(None)
}

/// Build and Test the PR. Result the Build-Test Result.
async fn build_test(pr: &PullRequest, target: &str, script: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Get the Head Ref and Head URL from PR
    // let pr: Value = serde_json::from_str(&body).unwrap();
    // let pr_id = pr["number"].as_u64().unwrap();
    let head = &pr.head;
    let head_ref = &head.ref_field;  // "test-bot"
    let head_url = head.repo.clone().unwrap().html_url.unwrap();  // https://github.com/lupyuen2/wip-nuttx
    println!("head_ref={head_ref}");
    println!("head_url={head_url}");

    // True if URL is an Apps Repo
    let is_apps =
        if head_url.as_str().contains("apps") { true }
        else { false };

    // Set the URLs and Refs for NuttX and Apps
    let nuttx_hash = "HEAD";
    let nuttx_url =
        if is_apps { "https://github.com/apache/nuttx" }
        else { head_url.as_str() };
    let nuttx_ref =
        if is_apps { "master" }
        else { head_ref };
    let apps_hash = "HEAD";
    let apps_url = 
        if is_apps { head_url.as_str() }
        else { "https://github.com/apache/nuttx-apps" };
    let apps_ref =
        if is_apps { head_ref }
        else { "master" };

    // Build and Test NuttX: ./build-test.sh knsh64 /tmp/build-test.log HEAD HEAD https://github.com/apache/nuttx master https://github.com/apache/nuttx-apps master
    // Which calls: ./build-test-knsh64.sh HEAD HEAD https://github.com/apache/nuttx master https://github.com/apache/nuttx-apps master
    let cmd = format!("./build-test-{script}.sh {nuttx_hash} {apps_hash} {nuttx_url} {nuttx_ref} {apps_url} {apps_ref}");
    println!("cmd={cmd}");
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

    // Extract the Log Output
    let log_extract = extract_log(&snippet_url).await?;
    let log_content = log_extract.join("\n");
    println!("log_content=\n{log_content}");
    let mut result = 
        if status.success() { format!("Build and Test Successful ({target})\n") }
        else { format!("Build and Test **FAILED** ({target})\n") };
    result.push_str(&snippet_url);
    result.push_str("\n```text\n");
    result.push_str(&log_content);
    result.push_str("\n```\n");
    println!("result={result}");

    // Return the Result
    Ok(result)
}

/// Extract the important bits from the Build / Test Log.
/// url looks like "https://gitlab.com/lupyuen/nuttx-build-log/-/snippets/4799962#L85"
async fn extract_log(url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // raw_url looks like "https://gitlab.com/lupyuen/nuttx-build-log/-/snippets/4799962/raw/"
    let parsed_url = Url::parse(url).unwrap();
    let start_line = parsed_url.fragment().unwrap_or("L1");  // "L85" ////
    let start_linenum = start_line[1..].parse::<usize>().unwrap();  // 85
    let mut parsed_url = parsed_url.clone();
    parsed_url.set_fragment(None); // "https://gitlab.com/lupyuen/nuttx-build-log/-/snippets/4799962"
    let base_url = parsed_url.as_str();  
    let raw_url = format!("{base_url}/raw/");
    println!("raw_url={raw_url}");

    // output_line[i] is True if Line #i should be extracted for output (starts at i=1)
    let log = reqwest::get(raw_url).await?
        .text().await?;
    // println!("log=\n{log}");
    let lines = &log.split('\n').collect::<Vec<_>>();
    let mut output_line = BitVec::from_elem(lines.len() + 1, false);

    // Extract Log from Start Line Number till "===== Error: Test Failed" or "===== Test OK"
    for (linenum, line) in lines.into_iter().enumerate() {
        if linenum < start_linenum { continue; }
        if line.starts_with("===== ") || linenum == lines.len() - 1 { ////
            // Extract the previous 10 lines
            for i in (linenum - 10)..linenum { output_line.set(i, true); }
            // for i in (linenum - 10)..linenum { println!("{}", lines[i]); }
            break;
        } else if 
            // Skip these lines
            line.contains("/nuttx-build-farm/") ||  // "/home/luppy/nuttx-build-farm/build-test-knsh64.sh 657247bda89d60112d79bb9b8d223eca5f9641b5 a6b9e718460a56722205c2a84a9b07b94ca664aa"
            line.starts_with("+ [[") ||  // "[[ 657247bda89d60112d79bb9b8d223eca5f9641b5 != '' ]]"
            line.starts_with("+ set ") ||  // "set +x"
            line.starts_with("+ nuttx_hash") || // "nuttx_hash=657247bda89d60112d79bb9b8d223eca5f9641b5"
            line.starts_with("+ apps_hash") || // "apps_hash=a6b9e718460a56722205c2a84a9b07b94ca664aa"
            line.starts_with("+ nuttx_url") || // "nuttx_url=https://github.com/apache/nuttx" ////
            line.starts_with("+ apps_url") || // "apps_url=https://github.com/apache/nuttx-apps" ////
            line.starts_with("+ nuttx_ref") || // "nuttx_ref=test-bot" ////
            line.starts_with("+ apps_ref") || // "apps_ref=master" ////
            line.starts_with("+ export ") || // "export OZ64_SERVER=tftpserver" ////
            line.starts_with("+ OZ64_SERVER") || // "OZ64_SERVER=tftpserver" ////
            line.starts_with("+ script_dir") || // "script_dir=/home/luppy/nuttx-build-farm" ////
            line.starts_with("+ neofetch") || // "neofetch"
            line.starts_with("+ tmp_path") || // "tmp_path=/tmp/build-test-knsh64"
            line.starts_with("+ rm -rf /tmp/") ||  // "rm -rf /tmp/build-test-knsh64"
            line.starts_with("+ mkdir /tmp/") ||  // "mkdir /tmp/build-test-knsh64"
            line.starts_with("+ cd /tmp/") ||  // "cd /tmp/build-test-knsh64"
            line.starts_with("+ riscv-none-elf-gcc -v") ||  // "riscv-none-elf-gcc -v"
            line.starts_with("+ rustup --version") ||  // "rustup --version"
            line.starts_with("+ rustc --version") ||  // "rustc --version"
            line.starts_with("+ riscv-none-elf-size") ||  // "riscv-none-elf-size nuttx"
            line.starts_with("+ script=") ||  // "script=qemu-riscv-knsh64"
            line.starts_with("+ wget ") ||  // "wget https://raw.githubusercontent.com/lupyuen/nuttx-riscv64/main/qemu-riscv-knsh64.exp"
            line.starts_with("+ expect ") ||  // "expect ./qemu-riscv-knsh64.exp"
            false {
            continue;
        } else if
            // Output these lines
            line.starts_with("+ ") ||
            line.starts_with("HEAD is now") ||  // "HEAD is now at 657247bda8 libc/modlib: preprocess gnu-elf.ld"
            line.starts_with("NuttX Source") ||  // "NuttX Source: https://github.com/apache/nuttx/tree/657247bda89d60112d79bb9b8d223eca5f9641b5"
            line.starts_with("NuttX Apps") ||  // "NuttX Apps: https://github.com/apache/nuttx-apps/tree/a6b9e718460a56722205c2a84a9b07b94ca664aa"
            line.contains("+ pushd ../apps") || // "CC:  ... + pushd ../apps"
            line.starts_with("spawn") ||  // "spawn qemu-system-riscv64 -semihosting -M virt,aclint=on -cpu rv64 -kernel nuttx -nographic"
            line.starts_with("QEMU emulator") ||  // "QEMU emulator version 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.4)"
            line.starts_with("OpenSBI") ||  // "OpenSBI v1.3"
            line.starts_with("nsh> uname") ||  // "nsh> uname -a" ////
            line.starts_with("NuttX") ||  // "NuttX 10.4.0 fa059c19fa Feb  5 2025 19:25:45 risc-v rv-virt" ////
            line.starts_with("nsh> ostest") ||  // "nsh> ostest" ////
            line.starts_with("ostest_main: Exiting") ||  // "ostest_main: Exiting with status 0" ////
            false {
            output_line.set(linenum, true);
            // println!("line={line}");
        }
    }

    // Consolidate the Extracted Log Lines
    let mut msg: Vec<String> = vec![];
    for (linenum, line) in lines.into_iter().enumerate() {
        if !output_line.get(linenum).unwrap() { continue; }
        let line =
            if line.contains("+ pushd ../apps") { "$ pushd ../apps".into() }  // "CC:  ... + pushd ../apps"
            else if line.starts_with("spawn ") { line.replace("spawn ", "$ ") }  // "spawn qemu-system-riscv64 -semihosting -M virt,aclint=on -cpu rv64 -kernel nuttx -nographic"
            else if line.starts_with("+ ") { "$ ".to_string() + &line[2..] }  // "+ " becomes "$ "
            else { line.to_string() };
        println!("{linenum}: {line}");
        msg.push(line);
    }
    Ok(msg)
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
