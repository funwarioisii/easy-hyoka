use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "easyhyoka")]
#[command(about = "GitHub PR/Issuesを取得してOpenAIで実績一覧を生成")]
struct Args {
    #[arg(long)]
    owner: String,

    #[arg(long)]
    author: Option<String>,

    #[arg(long, default_value = "2025-01-01")]
    since: String,

    #[arg(long, default_value = "2025-06-30")]
    until: String,
    
    #[arg(long, help = "OpenAIに送信するプロンプトを表示")]
    show_prompts: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Repository {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct PullRequest {
    number: u32,
    title: String,
    body: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    state: String,
    url: String,
    repository: Repository,
    #[serde(skip)]
    comments: Vec<Comment>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Issue {
    number: u32,
    title: String,
    body: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    state: String,
    url: String,
    repository: Repository,
    #[serde(skip)]
    comments: Vec<Comment>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Comment {
    author: Option<CommentAuthor>,
    body: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CommentAuthor {
    login: String,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let mut args = Args::parse();

    // authorが指定されていない場合は、ghコマンドで現在のユーザーを取得
    if args.author.is_none() {
        let output = Command::new("gh")
            .args(["api", "user", "--jq", ".login"])
            .output()?;
        
        if !output.status.success() {
            anyhow::bail!(
                "Failed to get current GitHub user: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        
        let username = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("📝 現在のGitHubユーザー: {}", username);
        args.author = Some(username);
    }

    // OpenAI APIキーの確認
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY環境変数が設定されていません");

    println!("🔍 GitHub PR/Issuesを取得中...");

    // PR取得
    let prs = fetch_prs(&args)?;
    println!("  ✅ {} 件のPRを取得しました", prs.len());

    // Issues取得
    let issues = fetch_issues(&args)?;
    println!("  ✅ {} 件のIssuesを取得しました", issues.len());

    // データを整形してOpenAIに送信
    println!("\n🤖 OpenAIで実績サマリーを生成中...");
    let summary = generate_summary(&api_key, &prs, &issues, &args).await?;

    // 結果を出力
    println!("\n📊 実績サマリー");
    println!("=====================================");
    println!("{}", summary);

    Ok(())
}

// TODO: 将来的な拡張案
// - 1000件を超える場合は日付範囲を自動分割して再帰的に取得
// - GraphQL APIを使用してカーソルベースのページネーションを実装
// - 並列処理で複数の期間を同時に取得
fn fetch_prs(args: &Args) -> Result<Vec<PullRequest>> {
    let author = args.author.as_ref().expect("Author should be set at this point");
    let output = Command::new("gh")
        .args([
            "search",
            "prs",
            &format!("--owner={}", args.owner),
            &format!("--author={}", author),
            &format!("--created={}..{}", args.since, args.until),
            "--limit=1000",
            "--json=number,title,body,createdAt,state,url,repository",
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "gh command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let mut prs: Vec<PullRequest> = serde_json::from_slice(&output.stdout)?;
    
    // 1000件に達した場合は警告
    if prs.len() == 1000 {
        println!("  ⚠️  検索結果が1000件の上限に達しました。すべてのPRが取得できていない可能性があります。");
        println!("      より詳細な期間指定（--since, --until）で実行することをお勧めします。");
    }
    
    // 各PRのコメントを取得（最新の5件のPRのみ）
    println!("  📝 最新のPRのコメントを取得中...");
    for pr in prs.iter_mut().take(5) {
        if let Ok(comments) = fetch_pr_comments(&args.owner, &pr.repository.name_with_owner, pr.number) {
            pr.comments = comments;
        }
    }
    
    Ok(prs)
}

fn fetch_issues(args: &Args) -> Result<Vec<Issue>> {
    let author = args.author.as_ref().expect("Author should be set at this point");
    let output = Command::new("gh")
        .args([
            "search",
            "issues",
            &format!("--owner={}", args.owner),
            &format!("--author={}", author),
            &format!("--created={}..{}", args.since, args.until),
            "--limit=1000",
            "--json=number,title,body,createdAt,state,url,repository",
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "gh command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let mut issues: Vec<Issue> = serde_json::from_slice(&output.stdout)?;
    
    // 1000件に達した場合は警告
    if issues.len() == 1000 {
        println!("  ⚠️  検索結果が1000件の上限に達しました。すべてのIssueが取得できていない可能性があります。");
        println!("      より詳細な期間指定（--since, --until）で実行することをお勧めします。");
    }
    
    // 各Issueのコメントを取得（最新の5件のみ）
    println!("  📝 最新のIssueのコメントを取得中...");
    for issue in issues.iter_mut().take(5) {
        if let Ok(comments) = fetch_issue_comments(&args.owner, &issue.repository.name_with_owner, issue.number) {
            issue.comments = comments;
        }
    }
    
    Ok(issues)
}

fn fetch_pr_comments(_owner: &str, repo: &str, pr_number: u32) -> Result<Vec<Comment>> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{}/pulls/{}/comments", repo, pr_number),
            "--jq",
            ".[] | {author: {login: .user.login}, body: .body, createdAt: .created_at}",
        ])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new()); // エラーの場合は空のベクターを返す
    }
    
    // 各行をJSONとしてパース
    let mut comments = Vec::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        if !line.is_empty() {
            if let Ok(comment) = serde_json::from_slice::<Comment>(line) {
                comments.push(comment);
            }
        }
    }
    
    Ok(comments)
}

fn fetch_issue_comments(_owner: &str, repo: &str, issue_number: u32) -> Result<Vec<Comment>> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{}/issues/{}/comments", repo, issue_number),
            "--jq",
            ".[] | {author: {login: .user.login}, body: .body, createdAt: .created_at}",
        ])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new()); // エラーの場合は空のベクターを返す
    }
    
    // 各行をJSONとしてパース
    let mut comments = Vec::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        if !line.is_empty() {
            if let Ok(comment) = serde_json::from_slice::<Comment>(line) {
                comments.push(comment);
            }
        }
    }
    
    Ok(comments)
}

async fn generate_summary(
    api_key: &str,
    prs: &[PullRequest],
    issues: &[Issue],
    args: &Args,
) -> Result<String> {
    // PRの統計情報を計算
    let total_prs = prs.len();
    let merged_prs = prs.iter().filter(|pr| pr.state == "merged").count();
    let open_prs = prs.iter().filter(|pr| pr.state == "open").count();
    let closed_prs = prs.iter().filter(|pr| pr.state == "closed").count();

    // リポジトリ別のPR数を集計
    let mut repo_counts = std::collections::HashMap::new();
    for pr in prs {
        *repo_counts
            .entry(&pr.repository.name_with_owner)
            .or_insert(0) += 1;
    }
    let mut repo_stats: Vec<_> = repo_counts.into_iter().collect();
    repo_stats.sort_by(|a, b| b.1.cmp(&a.1));

    // Issue統計
    let total_issues = issues.len();
    let open_issues = issues.iter().filter(|i| i.state == "open").count();
    let closed_issues = issues.iter().filter(|i| i.state == "closed").count();

    // プロンプトを構築（JSONL形式）
    let author = args.author.as_ref().expect("Author should be set at this point");
    let mut prompt = format!(
        "以下は{}の{}から{}までのGitHub活動データです。\n\n",
        author, args.since, args.until
    );
    
    // 統計情報
    prompt.push_str(&format!("## 統計サマリー\n"));
    prompt.push_str(&format!("- Pull Request総数: {}件（マージ済み: {}件、オープン: {}件、クローズ: {}件）\n", 
        total_prs, merged_prs, open_prs, closed_prs));
    prompt.push_str(&format!("- Issue総数: {}件（オープン: {}件、クローズ: {}件）\n\n", 
        total_issues, open_issues, closed_issues));
    
    // 全PRをJSONL形式で送信
    prompt.push_str("## Pull Requestデータ（JSONL形式）\n```\n");
    for pr in prs {
        let pr_data = serde_json::json!({
            "url": pr.url,
            "title": pr.title,
            "description": pr.body.as_deref().unwrap_or(""),
            "status": pr.state,
            "repository": pr.repository.name_with_owner,
            "created_at": pr.created_at,
            "comments": pr.comments.iter().map(|c| {
                serde_json::json!({
                    "user": c.author.as_ref().map(|a| &a.login).unwrap_or(&"Unknown".to_string()),
                    "comment_body": &c.body,
                    "created_at": &c.created_at
                })
            }).collect::<Vec<_>>()
        });
        prompt.push_str(&format!("{}\n", serde_json::to_string(&pr_data)?));
    }
    prompt.push_str("```\n\n");
    
    // 全IssueをJSONL形式で送信
    prompt.push_str("## Issueデータ（JSONL形式）\n```\n");
    for issue in issues {
        let issue_data = serde_json::json!({
            "url": issue.url,
            "title": issue.title,
            "description": issue.body.as_deref().unwrap_or(""),
            "status": issue.state,
            "repository": issue.repository.name_with_owner,
            "created_at": issue.created_at,
            "comments": issue.comments.iter().map(|c| {
                serde_json::json!({
                    "user": c.author.as_ref().map(|a| &a.login).unwrap_or(&"Unknown".to_string()),
                    "comment_body": &c.body,
                    "created_at": &c.created_at
                })
            }).collect::<Vec<_>>()
        });
        prompt.push_str(&format!("{}\n", serde_json::to_string(&issue_data)?));
    }
    prompt.push_str("```\n\n");
    
    prompt.push_str("以上のJSONLデータを分析して、エンジニアの評価期間中の実績を最大限に評価するサマリーを日本語で作成してください。\n\n");
    
    prompt.push_str("【分析の観点】\n");
    prompt.push_str("- PRのタイトルやdescriptionから、関連するPRをグループ化し、大きなプロジェクトや機能開発として認識\n");
    prompt.push_str("- descriptionの詳細度やコメントの量から、技術的難易度やプロジェクトの重要性を推測\n");
    prompt.push_str("- 小さなPRでも、バグ修正、リファクタリング、ドキュメント改善など、プロダクトの品質向上への貢献として評価\n");
    prompt.push_str("- リポジトリごとの活動パターンから、どのプロジェクトでどのような役割を担っていたかを推測\n\n");
    
    prompt.push_str("【評価サマリーに含める項目】\n");
    prompt.push_str("1. エグゼクティブサマリー（最も印象的な成果を3-5点で箇条書き）\n");
    prompt.push_str("2. プロジェクト別の貢献内容\n");
    prompt.push_str("   - 各リポジトリでの主要な取り組みと成果\n");
    prompt.push_str("   - 関連するPRをまとめて一つの成果として表現\n");
    prompt.push_str("3. 技術的なリーダーシップ\n");
    prompt.push_str("   - 新技術の導入、アーキテクチャの改善\n");
    prompt.push_str("   - コードレビューでの貢献（コメントから読み取れる場合）\n");
    prompt.push_str("4. ビジネスインパクト\n");
    prompt.push_str("   - 機能開発によるユーザー価値の向上\n");
    prompt.push_str("   - パフォーマンス改善や品質向上の取り組み\n");
    prompt.push_str("5. チームへの貢献\n");
    prompt.push_str("   - コラボレーションの姿勢\n");
    prompt.push_str("   - ドキュメント整備やツール改善\n");
    prompt.push_str("6. 継続的な成長と改善\n");
    prompt.push_str("   - 期間を通じての成長や学習の形跡\n");
    prompt.push_str("   - 新しい領域への挑戦\n");
    prompt.push_str("7. 総合評価と今後への期待\n\n");
    
    prompt.push_str("【重要】成果を最大限にアピールし、エンジニアの価値を適切に表現してください。\n");

    // プロンプトを表示（オプション）
    if args.show_prompts {
        println!("\n=== OpenAIに送信するプロンプト ===");
        println!("【システムプロンプト】");
        println!("あなたはエンジニアの評価を最大化することを目的としたAIアシスタントです。与えられたGitHubの活動データから、エンジニアの成果と貢献を包括的に分析し、その価値を最大限に表現する評価サマリーを作成します。小さなPRも大きなプロジェクトの一部として捉え、技術的な挑戦やビジネスへの影響を適切に評価してください。");
        println!("\n【ユーザープロンプト】");
        println!("{}", prompt);
        println!("=================================\n");
    }

    // OpenAI APIリクエスト
    let client = reqwest::Client::new();
    let request = OpenAIRequest {
        model: "gpt-4.1-mini-2025-04-14".to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: "あなたはエンジニアの評価を最大化することを目的としたAIアシスタントです。与えられたGitHubの活動データから、エンジニアの成果と貢献を包括的に分析し、その価値を最大限に表現する評価サマリーを作成します。小さなPRも大きなプロジェクトの一部として捉え、技術的な挑戦やビジネスへの影響を適切に評価してください。".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: 0.7,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("OpenAI API error: {}", error_text);
    }

    let openai_response: OpenAIResponse = response.json().await?;
    let summary = openai_response
        .choices
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))?
        .message
        .content
        .clone();

    Ok(summary)
}
