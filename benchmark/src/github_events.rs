use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub type GithubEvents<'a> = Vec<GithubEventElement<'a>>;

#[derive(Serialize, Deserialize)]
pub struct GithubEventElement<'a> {
    #[serde(borrow, rename = "type")]
    github_event_type: Cow<'a, str>,
    #[serde(borrow)]
    created_at: Cow<'a, str>,
    actor: Actor<'a>,
    repo: Repo<'a>,
    public: bool,
    payload: Payload<'a>,
    #[serde(borrow)]
    id: Cow<'a, str>,
    org: Option<Actor<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct Actor<'a> {
    #[serde(borrow)]
    gravatar_id: Cow<'a, str>,
    #[serde(borrow)]
    login: Cow<'a, str>,
    #[serde(borrow)]
    avatar_url: Cow<'a, str>,
    #[serde(borrow)]
    url: Cow<'a, str>,
    id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Payload<'a> {
    commits: Option<Vec<Commit<'a>>>,
    distinct_size: Option<u64>,
    #[serde(rename = "ref", borrow)]
    payload_ref: Option<Cow<'a, str>>,
    push_id: Option<u64>,
    #[serde(borrow)]
    head: Option<Cow<'a, str>>,
    #[serde(borrow)]
    before: Option<Cow<'a, str>>,
    size: Option<u64>,
    #[serde(borrow)]
    description: Option<Cow<'a, str>>,
    #[serde(borrow)]
    master_branch: Option<Cow<'a, str>>,
    #[serde(borrow)]
    ref_type: Option<Cow<'a, str>>,
    forkee: Option<Forkee<'a>>,
    #[serde(borrow)]
    action: Option<Cow<'a, str>>,
    issue: Option<Issue<'a>>,
    comment: Option<Comment<'a>>,
    pages: Option<Vec<Page<'a>>>,
}

#[derive(Serialize, Deserialize)]
pub struct Comment<'a> {
    #[serde(borrow)]
    user: User<'a>,
    #[serde(borrow)]
    url: Cow<'a, str>,
    #[serde(borrow)]
    issue_url: Cow<'a, str>,
    #[serde(borrow)]
    created_at: Cow<'a, str>,
    #[serde(borrow)]
    body: Cow<'a, str>,
    #[serde(borrow)]
    updated_at: Cow<'a, str>,
    id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct User<'a> {
    #[serde(borrow)]
    url: Cow<'a, str>,
    #[serde(borrow)]
    gists_url: Cow<'a, str>,
    #[serde(borrow)]
    gravatar_id: Cow<'a, str>,
    #[serde(borrow, rename = "type")]
    user_type: Cow<'a, str>,
    #[serde(borrow)]
    avatar_url: Cow<'a, str>,
    #[serde(borrow)]
    subscriptions_url: Cow<'a, str>,
    #[serde(borrow)]
    received_events_url: Cow<'a, str>,
    #[serde(borrow)]
    organizations_url: Cow<'a, str>,
    #[serde(borrow)]
    repos_url: Cow<'a, str>,
    #[serde(borrow)]
    login: Cow<'a, str>,
    id: u64,
    #[serde(borrow)]
    starred_url: Cow<'a, str>,
    #[serde(borrow)]
    events_url: Cow<'a, str>,
    #[serde(borrow)]
    followers_url: Cow<'a, str>,
    #[serde(borrow)]
    following_url: Cow<'a, str>,
}

#[derive(Serialize, Deserialize)]
pub struct Commit<'a> {
    #[serde(borrow)]
    url: Cow<'a, str>,
    #[serde(borrow)]
    message: Cow<'a, str>,
    distinct: bool,
    #[serde(borrow)]
    sha: Cow<'a, str>,
    author: Author<'a>,
}

#[derive(Serialize, Deserialize)]
pub struct Author<'a> {
    #[serde(borrow)]
    email: Cow<'a, str>,
    #[serde(borrow)]
    name: Cow<'a, str>,
}

#[derive(Serialize, Deserialize)]
pub struct Forkee<'a> {
    #[serde(borrow)]
    description: Cow<'a, str>,
    fork: bool,
    #[serde(borrow)]
    url: Cow<'a, str>,
    #[serde(borrow)]
    language: Cow<'a, str>,
    #[serde(borrow)]
    stargazers_url: Cow<'a, str>,
    #[serde(borrow)]
    clone_url: Cow<'a, str>,
    #[serde(borrow)]
    tags_url: Cow<'a, str>,
    #[serde(borrow)]
    full_name: Cow<'a, str>,
    #[serde(borrow)]
    merges_url: Cow<'a, str>,
    forks: u64,
    private: bool,
    #[serde(borrow)]
    git_refs_url: Cow<'a, str>,
    #[serde(borrow)]
    archive_url: Cow<'a, str>,
    #[serde(borrow)]
    collaborators_url: Cow<'a, str>,
    owner: User<'a>,
    #[serde(borrow)]
    languages_url: Cow<'a, str>,
    #[serde(borrow)]
    trees_url: Cow<'a, str>,
    #[serde(borrow)]
    labels_url: Cow<'a, str>,
    #[serde(borrow)]
    html_url: Cow<'a, str>,
    #[serde(borrow)]
    pushed_at: Cow<'a, str>,
    #[serde(borrow)]
    created_at: Cow<'a, str>,
    has_issues: bool,
    #[serde(borrow)]
    forks_url: Cow<'a, str>,
    #[serde(borrow)]
    branches_url: Cow<'a, str>,
    #[serde(borrow)]
    commits_url: Cow<'a, str>,
    #[serde(borrow)]
    notifications_url: Cow<'a, str>,
    open_issues: u64,
    #[serde(borrow)]
    contents_url: Cow<'a, str>,
    #[serde(borrow)]
    blobs_url: Cow<'a, str>,
    #[serde(borrow)]
    issues_url: Cow<'a, str>,
    #[serde(borrow)]
    compare_url: Cow<'a, str>,
    #[serde(borrow)]
    issue_events_url: Cow<'a, str>,
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    updated_at: Cow<'a, str>,
    #[serde(borrow)]
    statuses_url: Cow<'a, str>,
    forks_count: u64,
    #[serde(borrow)]
    assignees_url: Cow<'a, str>,
    #[serde(borrow)]
    ssh_url: Cow<'a, str>,
    public: bool,
    has_wiki: bool,
    #[serde(borrow)]
    subscribers_url: Cow<'a, str>,
    mirror_url: (),
    watchers_count: u64,
    id: u64,
    has_downloads: bool,
    #[serde(borrow)]
    git_commits_url: Cow<'a, str>,
    #[serde(borrow)]
    downloads_url: Cow<'a, str>,
    #[serde(borrow)]
    pulls_url: Cow<'a, str>,
    #[serde(borrow)]
    homepage: Option<Cow<'a, str>>,
    #[serde(borrow)]
    issue_comment_url: Cow<'a, str>,
    #[serde(borrow)]
    hooks_url: Cow<'a, str>,
    #[serde(borrow)]
    subscription_url: Cow<'a, str>,
    #[serde(borrow)]
    milestones_url: Cow<'a, str>,
    #[serde(borrow)]
    svn_url: Cow<'a, str>,
    #[serde(borrow)]
    events_url: Cow<'a, str>,
    #[serde(borrow)]
    git_tags_url: Cow<'a, str>,
    #[serde(borrow)]
    teams_url: Cow<'a, str>,
    #[serde(borrow)]
    comments_url: Cow<'a, str>,
    open_issues_count: u64,
    #[serde(borrow)]
    keys_url: Cow<'a, str>,
    #[serde(borrow)]
    git_url: Cow<'a, str>,
    #[serde(borrow)]
    contributors_url: Cow<'a, str>,
    size: u64,
    watchers: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Issue<'a> {
    user: User<'a>,
    #[serde(borrow)]
    url: Cow<'a, str>,
    labels: Vec<()>,
    #[serde(borrow)]
    html_url: Cow<'a, str>,
    #[serde(borrow)]
    labels_url: Cow<'a, str>,
    pull_request: PullRequest,
    #[serde(borrow)]
    created_at: Cow<'a, str>,
    #[serde(borrow)]
    closed_at: Option<Cow<'a, str>>,
    milestone: (),
    #[serde(borrow)]
    title: Cow<'a, str>,
    #[serde(borrow)]
    body: Cow<'a, str>,
    #[serde(borrow)]
    updated_at: Cow<'a, str>,
    number: u64,
    #[serde(borrow)]
    state: Cow<'a, str>,
    assignee: Option<User<'a>>,
    id: u64,
    #[serde(borrow)]
    events_url: Cow<'a, str>,
    #[serde(borrow)]
    comments_url: Cow<'a, str>,
    comments: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PullRequest {
    html_url: (),
    patch_url: (),
    diff_url: (),
}

#[derive(Serialize, Deserialize)]
pub struct Page<'a> {
    #[serde(borrow)]
    page_name: Cow<'a, str>,
    #[serde(borrow)]
    html_url: Cow<'a, str>,
    #[serde(borrow)]
    title: Cow<'a, str>,
    #[serde(borrow)]
    sha: Cow<'a, str>,
    summary: (),
    #[serde(borrow)]
    action: Cow<'a, str>,
}
#[derive(Serialize, Deserialize)]
pub struct Repo<'a> {
    #[serde(borrow)]
    url: Cow<'a, str>,
    id: u64,
    #[serde(borrow)]
    name: Cow<'a, str>,
}
