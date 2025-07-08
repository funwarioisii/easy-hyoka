# easy-hyoka

GitHub活動からAIで評価サマリーを生成する

## セットアップ

```bash
# 依存
brew install gh
gh auth login

# インストール
cargo install --git https://github.com/funwarioisii/easy-hyoka

# 環境変数
export OPENAI_API_KEY=your-key
```

## 使用方法

```bash
easy-hyoka --owner=org-name
```

## ライセンス

MIT
