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

    // Get the Handlers for GitHub Pull Requests and Issues
    let pulls = octocrab.pulls(&args.owner, &args.repo);
    let issues = octocrab.issues(&args.owner, &args.repo);

    // Fetch the 20 Newest Pull Requests that are Open
    let pr_list = pulls
        .list()
        .state(params::State::Open)
        .sort(params::pulls::Sort::Created)
        .direction(params::Direction::Descending)
        .per_page(20)
        .send()
        .await?;

    // Every 5 Seconds: Process the next PR fetched
    for pr in pr_list {
        let pr_id = pr.number;
        process_pr(&pulls, &issues, pr_id)
            .await?;
        sleep(Duration::from_secs(5));
    }

    // Return OK
    Ok(())
}

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
