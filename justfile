# Command Runner
# Run `just` or `just --list` to see available commands

# Default recipe - show available commands
default:
    @just --list

# ══════════════════════════════════════════════════════════════════════════════
# Setup
# ══════════════════════════════════════════════════════════════════════════════

# Bootstrap the development environment (idempotent - safe to run multiple times)
[group('Setup')]
bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "🚀 Bootstrapping development environment..."
    echo ""

    # Configure git hooks (idempotent - git config overwrites existing value)
    echo "🔧 Configuring git hooks..."
    git config core.hooksPath .githooks
    echo "✅ Git hooks configured (using .githooks/)"
    echo ""

    # Check required tools
    echo "📋 Checking required tools..."
    MISSING=0
    command -v cargo >/dev/null 2>&1 || { echo "❌ cargo not found (install via mise or rustup)"; MISSING=1; }
    command -v kingfisher >/dev/null 2>&1 || { echo "❌ kingfisher not found (brew install kingfisher)"; MISSING=1; }
    [ $MISSING -eq 0 ] && echo "✅ All required tools found"
    echo ""

    echo "🎉 Bootstrap complete!"

# ══════════════════════════════════════════════════════════════════════════════
# Development
# ══════════════════════════════════════════════════════════════════════════════

# Format all code
[group('Development')]
fmt:
    cargo fmt --all

# Run clippy linter
[group('Development')]
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
[group('Development')]
test:
    cargo test --all

# Run tests with coverage report
# Ignores files that can't be meaningfully unit tested:
#   main.rs (terminal wiring), event.rs (crossterm wrapper),
#   theme.rs (static data), view.rs (rendering, needs a frame)
[group('Development')]
coverage:
    cargo llvm-cov --all \
        --ignore-filename-regex '(main|app|event|theme|view)\.rs$'

# Format and lint
[group('Development')]
check: fmt lint

# ══════════════════════════════════════════════════════════════════════════════
# GitHub
# ══════════════════════════════════════════════════════════════════════════════

# Apply standard branch protection + security settings to a weld-rs repo (idempotent)
# Usage: just protect-repo weld-tui
[group('GitHub')]
protect-repo repo:
    #!/usr/bin/env bash
    set -euo pipefail
    REPO="weld-rs/{{repo}}"
    echo "🛡️  Applying protections to $REPO..."

    # Helper: create or update a ruleset by name (idempotent)
    apply_ruleset() {
        local name="$1"
        local json="$2"
        local existing
        existing=$(gh api "/repos/$REPO/rulesets" --jq ".[] | select(.name == \"$name\") | .id" 2>/dev/null || true)
        if [ -n "$existing" ]; then
            echo "  ↻ updating '$name' (id=$existing)"
            echo "$json" | gh api -X PUT "/repos/$REPO/rulesets/$existing" --input - >/dev/null
        else
            echo "  + creating '$name'"
            echo "$json" | gh api -X POST "/repos/$REPO/rulesets" --input - >/dev/null
        fi
    }

    # 1a. Remove the legacy single-ruleset "main" if present (replaced by the pair below)
    LEGACY_ID=$(gh api "/repos/$REPO/rulesets" --jq '.[] | select(.name == "main") | .id' 2>/dev/null || true)
    if [ -n "$LEGACY_ID" ]; then
        echo "  ✗ removing legacy 'main' ruleset (id=$LEGACY_ID)"
        gh api -X DELETE "/repos/$REPO/rulesets/$LEGACY_ID" >/dev/null
    fi

    # 1b. Baseline ruleset: applies to ALL branches
    ALL_BRANCHES_JSON=$(cat <<'JSON'
    {
      "name": "all-branches",
      "target": "branch",
      "enforcement": "active",
      "conditions": {
        "ref_name": {"include": ["~ALL"], "exclude": []}
      },
      "bypass_actors": [
        {"actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always"}
      ],
      "rules": [
        {"type": "non_fast_forward"},
        {"type": "required_signatures"}
      ]
    }
    JSON
    )
    apply_ruleset "all-branches" "$ALL_BRANCHES_JSON"

    # 1c. Hardening ruleset: applies only to the default branch
    DEFAULT_BRANCH_JSON=$(cat <<'JSON'
    {
      "name": "default-branch",
      "target": "branch",
      "enforcement": "active",
      "conditions": {
        "ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}
      },
      "bypass_actors": [
        {"actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always"}
      ],
      "rules": [
        {"type": "deletion"},
        {"type": "required_linear_history"},
        {
          "type": "pull_request",
          "parameters": {
            "required_approving_review_count": 1,
            "dismiss_stale_reviews_on_push": true,
            "require_code_owner_review": false,
            "require_last_push_approval": false,
            "required_review_thread_resolution": true,
            "allowed_merge_methods": ["squash"]
          }
        }
      ]
    }
    JSON
    )
    apply_ruleset "default-branch" "$DEFAULT_BRANCH_JSON"

    # 2. Secret scanning + push protection
    gh api -X PATCH "/repos/$REPO" \
        -f 'security_and_analysis[secret_scanning][status]=enabled' \
        -f 'security_and_analysis[secret_scanning_push_protection][status]=enabled' >/dev/null
    echo "  ✓ secret scanning + push protection"

    # 3. Merge settings: squash-only, auto-delete branch
    gh api -X PATCH "/repos/$REPO" \
        -F 'allow_squash_merge=true' \
        -F 'allow_merge_commit=false' \
        -F 'allow_rebase_merge=false' \
        -F 'delete_branch_on_merge=true' >/dev/null
    echo "  ✓ squash-only merges + auto-delete branches"

    echo ""
    echo "✅ $REPO protected"
    echo ""
    echo "⚠️  Manual per-repo follow-ups (require repo-specific setup):"
    echo "   • Add required status checks to the ruleset once CI exists"
    echo "   • Enable CodeQL (Security → Code scanning) + add to ruleset"
    echo "   • Settings → Actions → General → Fork pull request workflows:"
    echo "     'Require approval for first-time contributors'"
    echo "   • Install Renovate app + add .github/renovate.json"

# Show unresolved, non-outdated review comments on a PR
[group('GitHub')]
pr-status pr:
    #!/usr/bin/env bash
    set -euo pipefail
    RESULT=$(gh api graphql -f query='
    {
      repository(owner: "robwilkerson", name: "weld-tui") {
        pullRequest(number: '"{{pr}}"') {
          reviewThreads(first: 50) {
            nodes {
              isResolved
              isOutdated
              comments(first: 1) {
                nodes {
                  path
                  line
                  body
                }
              }
            }
          }
        }
      }
    }' --jq '.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved == false and .isOutdated == false) | "\(.comments.nodes[0].path):\(.comments.nodes[0].line) — \(.comments.nodes[0].body | split("\n")[0])"')
    if [ -z "$RESULT" ]; then
        echo "✅ No unresolved comments"
    else
        echo "⚠️  Unresolved comments:"
        echo "$RESULT"
    fi
