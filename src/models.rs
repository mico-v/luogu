#![allow(non_snake_case)]
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ── Existing types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemRecord {
    pub pid: String,
    pub title: String,
    pub difficulty: Option<i32>,
    pub difficulty_label: String,
    pub time_limit_ms: Option<i64>,
    pub memory_limit_kb: Option<i64>,
    pub tags: Vec<i64>,
    pub fetched_at: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeCompileInfo {
    pub success: bool,
    pub elapsed_seconds: f64,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeTestResult {
    pub name: String,
    pub status: String,
    pub time_ms: Option<f64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeLogEntry {
    pub timestamp: String,
    pub pid: String,
    pub status: String,
    pub success: bool,
    pub pass_count: usize,
    pub test_count: usize,
    pub compile: JudgeCompileInfo,
    pub tests: Vec<JudgeTestResult>,
}

#[derive(Debug, Clone)]
pub struct ParsedProblem {
    pub pid: String,
    pub title: String,
    pub difficulty: Option<i32>,
    pub limits_time_ms: Option<i64>,
    pub limits_memory_kb: Option<i64>,
    pub tags: Vec<i64>,
    pub markdown: String,
    pub samples: Vec<(String, String)>,
    pub provider: Option<String>,
    pub total_accepted: Option<i64>,
    pub total_submit: Option<i64>,
}

// ── API wrapper types ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LentilleResponse<T> {
    pub data: T,
    pub user: Option<serde_json::Value>,
    #[serde(deserialize_with = "deserialize_f64_as_i64")]
    pub time: i64,
}

#[derive(Debug, Deserialize)]
pub struct DataResponse<T> {
    pub currentData: T,
    pub code: i64,
}

// ── Problem API ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ProblemData {
    pub problem: ProblemDetails,
    pub bookmarked: bool,
    pub recommendations: Vec<ProblemSummary>,
    pub translations: serde_json::Value,
    pub lastLanguage: Option<i64>,
    pub lastCode: Option<String>,
    pub canEdit: bool,
}

#[derive(Debug, Deserialize)]
pub struct ProblemDetails {
    pub pid: String,
    pub title: String,
    pub difficulty: Option<i64>,
    pub tags: Vec<i64>,
    pub content: ProblemContent,
    pub samples: Vec<(String, String)>,
    pub limits: ProblemLimits,
    pub provider: Option<serde_json::Value>,
    pub totalSubmit: Option<i64>,
    pub totalAccepted: Option<i64>,
    pub flag: Option<i64>,
    pub wantsTranslation: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ProblemContent {
    pub name: String,
    pub background: Option<String>,
    pub description: Option<String>,
    pub formatI: Option<String>,
    pub formatO: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProblemLimits {
    pub time: Vec<i64>,
    pub memory: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProblemSummary {
    pub pid: String,
    pub title: String,
    pub difficulty: Option<i64>,
    #[serde(rename = "type")]
    pub ptype: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProblemListParams {
    pub page: Option<i64>,
    pub keyword: Option<String>,
    pub difficulty: Option<i64>,
    #[serde(rename = "type")]
    pub ptype: Option<String>,
    pub tag: Option<String>,
    pub orderBy: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProblemListData {
    pub result: Vec<LegacyProblemWithStatus>,
    pub count: i64,
    pub perPage: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyProblemSummary {
    pub pid: String,
    pub title: String,
    pub difficulty: Option<i64>,
    pub fullScore: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyProblem {
    pub pid: String,
    #[serde(alias = "name")]
    pub title: String,
    pub difficulty: Option<i64>,
    pub tags: Vec<i64>,
    pub totalSubmit: Option<i64>,
    pub totalAccepted: Option<i64>,
    pub flag: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MaybeProblemStatus {
    pub submitted: Option<bool>,
    pub accepted: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyProblemWithStatus {
    pub pid: String,
    #[serde(alias = "name")]
    pub title: String,
    pub difficulty: Option<i64>,
    pub tags: Option<Vec<i64>>,
    pub totalSubmit: Option<i64>,
    pub totalAccepted: Option<i64>,
    pub flag: Option<i64>,
    pub submitted: Option<bool>,
    pub accepted: Option<bool>,
    pub fullScore: Option<i64>,
}

// ── Training API ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ProblemSetData {
    pub training: ProblemSetDetails,
    pub canEdit: bool,
}

#[derive(Debug, Deserialize)]
pub struct ProblemSetDetails {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub ptype: i64,
    pub provider: serde_json::Value,
    pub createTime: i64,
    pub problemCount: i64,
    pub description: String,
    pub marked: Option<bool>,
    pub markCount: Option<i64>,
    pub problems: Vec<LegacyProblem>,
}

#[derive(Debug, Deserialize)]
pub struct ProblemSetProblemList {
    pub result: Vec<ProblemSetProblemEntry>,
    pub count: i64,
    pub perPage: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ProblemSetProblemEntry {
    pub problem: LegacyProblem,
}

#[derive(Debug, Deserialize)]
pub struct ProblemSetListParams {
    pub page: Option<i64>,
    pub keyword: Option<String>,
    #[serde(rename = "type")]
    pub ptype: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProblemSetListData {
    pub trainings: ProblemSetList,
    #[serde(alias = "acceptedCounts")]
    pub acCounts: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ProblemSetList {
    #[serde(deserialize_with = "deserialize_list_result")]
    pub result: Vec<ProblemSet>,
    pub count: i64,
    pub perPage: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ProblemSet {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub ptype: i64,
    pub problemCount: i64,
    pub markCount: i64,
    pub createTime: i64,
}

// ── Contest API ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ContestData {
    pub contest: ContestDetails,
    pub contestProblems: Option<Vec<ContestProblemEntry>>,
    pub canEdit: bool,
    pub joined: i64,
}

#[derive(Debug, Deserialize)]
pub struct ContestDetails {
    pub id: i64,
    pub name: String,
    pub startTime: i64,
    pub endTime: i64,
    pub description: Option<String>,
    pub problemCount: i64,
    pub totalParticipants: i64,
    pub host: serde_json::Value,
    pub visibility: i64,
    pub method: i64,
    pub rated: serde_json::Value,
    pub eloThreshold: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ContestProblemEntry {
    pub score: i64,
    pub problem: ProblemSummary,
}

#[derive(Debug, Deserialize)]
pub struct ContestListParams {
    pub page: Option<i64>,
    pub name: Option<String>,
    pub method: Option<i64>,
    #[serde(rename = "public")]
    pub public: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ContestListData {
    pub contests: ContestList,
}

#[derive(Debug, Deserialize)]
pub struct ContestList {
    #[serde(deserialize_with = "deserialize_list_result")]
    pub result: Vec<ContestSummary>,
    pub count: i64,
    pub perPage: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ContestSummary {
    pub id: i64,
    pub name: String,
    pub startTime: i64,
    pub endTime: i64,
    pub method: i64,
    pub visibility: i64,
    pub host: serde_json::Value,
    pub problemCount: i64,
    pub rated: serde_json::Value,
}

// ── User API ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UserData {
    pub user: UserDetails,
    pub elo: Vec<EloRating>,
    pub gu: GuRating,
    pub dailyCounts: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UserDetails {
    pub uid: i64,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub slogan: Option<String>,
    pub badge: Option<String>,
    pub isAdmin: bool,
    pub isBanned: bool,
    pub ccfLevel: i64,
    pub followingCount: i64,
    pub followerCount: i64,
    pub ranking: Option<i64>,
    pub eloValue: Option<i64>,
    pub registerTime: i64,
    pub introduction: Option<String>,
    pub passedProblemCount: Option<i64>,
    pub submittedProblemCount: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct GuRating {
    pub rating: i64,
    pub time: i64,
    pub scores: GuRatingScores,
}

#[derive(Debug, Deserialize)]
pub struct GuRatingScores {
    pub rating: i64,
    pub social: i64,
    pub basic: i64,
    pub contest: i64,
    pub practice: i64,
    pub prize: i64,
}

#[derive(Debug, Deserialize)]
pub struct EloRating {
    pub rating: i64,
    pub time: i64,
    pub latest: bool,
    pub userCount: Option<i64>,
    pub prevDiff: Option<i64>,
    pub contest: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UserSearchResult {
    pub users: Vec<Option<UserSummary>>,
}

#[derive(Debug, Deserialize)]
pub struct UserSummary {
    pub uid: i64,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub slogan: Option<String>,
    pub badge: Option<String>,
    pub isAdmin: bool,
    pub isBanned: bool,
    pub ccfLevel: i64,
}

// ── Solution API ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SolutionsData {
    pub solutions: SolutionList,
    pub problem: ProblemSummary,
}

#[derive(Debug, Deserialize)]
pub struct SolutionList {
    #[serde(deserialize_with = "deserialize_list_result")]
    pub result: Vec<ArticleDetails>,
    pub count: i64,
    pub perPage: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleDetails {
    pub lid: String,
    pub title: String,
    pub content: Option<String>,
    pub author: UserSummary,
    pub time: i64,
    pub upvote: i64,
    pub replyCount: i64,
    pub status: i64,
}

// ── Record API ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RecordListData {
    pub records: RecordList,
}

#[derive(Debug, Deserialize)]
pub struct RecordList {
    #[serde(deserialize_with = "deserialize_list_result")]
    pub result: Vec<RecordBase>,
    pub count: i64,
    pub perPage: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RecordBase {
    pub id: i64,
    pub status: i64,
    pub score: Option<i64>,
    pub time: Option<i64>,
    pub memory: Option<i64>,
    pub submitTime: i64,
    pub language: i64,
    pub enableO2: bool,
    pub problem: LegacyProblemSummary,
    pub contest: Option<serde_json::Value>,
    pub user: Option<UserSummary>,
}

#[derive(Debug, Deserialize)]
pub struct RecordData {
    pub record: RecordDetails,
    pub testCaseGroup: serde_json::Value,
    pub showStatus: bool,
}

#[derive(Debug, Deserialize)]
pub struct RecordDetails {
    pub id: i64,
    pub status: i64,
    pub score: Option<i64>,
    pub time: Option<i64>,
    pub memory: Option<i64>,
    pub submitTime: i64,
    pub language: i64,
    pub enableO2: bool,
    pub problem: LegacyProblemSummary,
    pub sourceCode: Option<String>,
    pub detail: Option<RecordStatus>,
    pub user: Option<UserSummary>,
    pub contest: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RecordStatus {
    pub compileResult: Option<CompileResult>,
    pub judgeResult: Option<JudgeResult>,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub message: Option<String>,
    pub opt2: bool,
}

#[derive(Debug, Deserialize)]
pub struct JudgeResult {
    pub subtasks: serde_json::Value,
    pub finishedCaseCount: i64,
    pub status: i64,
    pub time: i64,
    pub memory: i64,
    pub score: i64,
}

#[derive(Debug, Deserialize)]
pub struct RecordListParams {
    pub page: Option<i64>,
    pub pid: Option<String>,
    pub user: Option<String>,
    pub status: Option<i64>,
    pub language: Option<i64>,
}

// ── Tags API ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TagsResponse {
    pub tags: Vec<Tag>,
    pub types: Vec<TagType>,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub ptype: i64,
    pub parent: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TagType {
    pub id: i64,
    pub name: String,
    pub color: String,
}

// ── Ranking API ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RankingData {
    pub ranking: RankingList,
}

#[derive(Debug, Deserialize)]
pub struct RankingList {
    #[serde(deserialize_with = "deserialize_list_result")]
    pub result: Vec<GuRatingDetails>,
    pub count: i64,
    pub perPage: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct GuRatingDetails {
    pub user: UserSummary,
    pub rating: i64,
    pub time: i64,
    pub scores: GuRatingScores,
}

#[derive(Debug, Deserialize)]
pub struct EloRankingData {
    pub ranking: EloRankingList,
}

#[derive(Debug, Deserialize)]
pub struct EloRankingList {
    #[serde(deserialize_with = "deserialize_list_result")]
    pub result: Vec<EloRatingDetails>,
    pub count: i64,
    pub perPage: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct EloRatingDetails {
    pub user: UserSummary,
    pub rating: i64,
    pub time: i64,
    pub previous: Option<Box<EloRatingDetails>>,
    pub userCount: Option<i64>,
    pub prevDiff: Option<i64>,
}

// ── Submit API ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SubmitCodeRequest {
    pub code: String,
    pub lang: Option<i64>,
    pub enableO2: Option<i64>,
    pub captcha: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitResponse {
    pub rid: i64,
}

// ── Config API ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConfigResponse {
    pub codeLanguages: serde_json::Value,
    pub problemDifficulty: Vec<DifficultyInfo>,
    pub recordStatus: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DifficultyInfo {
    pub id: i64,
    pub name: String,
    pub color: String,
}

// ── Helper: deserialize list result that can be array or object ──

fn deserialize_list_result<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(T::deserialize)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::custom),
        serde_json::Value::Object(_) => Ok(Vec::new()),
        _ => Err(Error::custom("expected array or object for list result")),
    }
}

/// Helper: deserialize an f64 or i64 as i64 (for API time fields that can be float or int)
fn deserialize_f64_as_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => {
            n.as_i64()
                .or_else(|| n.as_f64().map(|f| f as i64))
                .ok_or_else(|| Error::custom("expected number"))
        }
        serde_json::Value::String(s) => {
            s.parse::<i64>().map_err(|_| Error::custom("expected number string"))
        }
        _ => Err(Error::custom("expected number for time field")),
    }
}