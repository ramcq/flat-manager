use actix::{Actor, SyncContext};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::mem;
use serde_json;

use chrono;
use schema::{ builds, build_refs, jobs, job_dependencies };

pub struct DbExecutor(pub Pool<ConnectionManager<PgConnection>>);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

#[derive(Identifiable, Serialize, Queryable, Debug, PartialEq)]
pub struct Build {
    pub id: i32,
    pub created: chrono::NaiveDateTime,
    pub repo_state: i16,
    pub repo_state_reason: Option<String>,
    pub published_state: i16,
    pub published_state_reason: Option<String>,
}

#[derive(Deserialize, Debug,PartialEq)]
pub enum PublishedState {
    Unpublished,
    Publishing,
    Published,
    Failed(String),
}

impl PublishedState {
    pub fn same_state_as(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }

    pub fn to_db(&self) -> (i16, Option<String>) {
        match self {
            PublishedState::Unpublished => (0, None),
            PublishedState::Publishing => (1, None),
            PublishedState::Published => (2, None),
            PublishedState::Failed(s) => (3, Some(s.to_string()))
        }
    }

    pub fn from_db(val: i16, reason: &Option<String>) -> Self {
        match val {
            0 => PublishedState::Unpublished,
            1 => PublishedState::Publishing,
            2 => PublishedState::Published,
            3 => PublishedState::Failed(reason.as_ref().unwrap_or(&"Unknown reason".to_string()).to_string()),
            _ => PublishedState::Failed("Unknown state".to_string()),
        }
    }
}

#[derive(Deserialize, Debug)]
pub enum RepoState {
    Uploading,
    Verifying,
    Ready,
    Failed(String),
    Purged,
}

impl RepoState {
    pub fn same_state_as(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }

    pub fn to_db(&self) -> (i16, Option<String>) {
        match self {
            RepoState::Uploading => (0, None),
            RepoState::Verifying => (1, None),
            RepoState::Ready => (2, None),
            RepoState::Failed(s) => (3, Some(s.to_string())),
            RepoState::Purged => (4, None),
        }
    }

    pub fn from_db(val: i16, reason: &Option<String>) -> Self {
        match val {
            0 => RepoState::Uploading,
            1 => RepoState::Verifying,
            2 => RepoState::Ready,
            3 => RepoState::Failed(reason.as_ref().unwrap_or(&"Unknown reason".to_string()).to_string()),
            4 => RepoState::Purged,
            _ => RepoState::Failed("Unknown state".to_string()),
        }
    }
}

#[derive(Deserialize, Insertable, Debug)]
#[table_name = "build_refs"]
pub struct NewBuildRef {
    pub build_id: i32,
    pub ref_name: String,
    pub commit: String,
}

#[derive(Identifiable, Associations, Serialize, Queryable, PartialEq, Debug)]
#[belongs_to(Build)]
pub struct BuildRef {
    pub id: i32,
    pub build_id: i32,
    pub ref_name: String,
    pub commit: String,
}

table! {
    job_dependencies_with_status (job_id, depends_on) {
        job_id -> Int4,
        depends_on -> Int4,
        dependant_status -> Int2,
    }
}

allow_tables_to_appear_in_same_query!(
    jobs,
    job_dependencies_with_status,
);

#[derive(Deserialize, Debug,PartialEq)]
pub enum JobStatus {
    New,
    Started,
    Ended,
    Broken,
}

#[derive(Debug,PartialEq)]
pub enum JobKind {
    Commit,
}

impl JobKind {
    pub fn _to_db(&self) -> i16 {
        match self {
            JobKind::Commit => 0,
        }
    }

    pub fn from_db(val: i16) -> Option<Self> {
        match val {
            0 => Some(JobKind::Commit),
            _ => None,
        }
    }
}

#[derive(Deserialize, Insertable, Debug)]
#[table_name = "jobs"]
pub struct NewJob {
    pub kind: i16,
    pub contents: serde_json::Value,
}

#[derive(Identifiable, Serialize, Queryable, Debug, PartialEq)]
pub struct Job {
    pub id: i32,
    pub kind: i16,
    pub status: i16,
    pub contents: serde_json::Value,
    pub results: Option<serde_json::Value>,
}

#[derive(Debug, Queryable, Identifiable, Associations)]
#[table_name = "job_dependencies"]
#[primary_key(job_id, depends_on)]
#[belongs_to(Job, foreign_key = "job_id")]
pub struct JobDependency {
    pub job_id: i32,
    pub depends_on: i32,
}

#[derive(Debug, Queryable, Identifiable, Associations)]
#[table_name = "job_dependencies_with_status"]
#[primary_key(job_id, depends_on)]
#[belongs_to(Job, foreign_key = "job_id")]
pub struct JobDependencyWithStatus {
    pub job_id: i32,
    pub depends_on: i32,
    pub dependant_status: i16,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct CommitJob {
    pub build: i32,
    pub endoflife: Option<String>,
}
