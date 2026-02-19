# Quality Check and Commit

Run quality checks (formatting, linting, tests) and commit changes if all checks pass.

## Instructions

1. Run `make all` to execute all quality checks:
   - Format check with Ruff
   - Strict Ruff linting on library code
   - All tests

2. If quality checks pass:
   - Review the changes using `git status` and `git diff`
   - Analyze the changes to create a meaningful commit message
   - Stage all relevant files with `git add`
   - Create a commit with a descriptive message that:
     - Summarizes the nature of changes (feature, fix, refactor, etc.)
     - Focuses on the "why" rather than the "what"
     - Follows the repository's commit message style
   - Append the standard Claude Code attribution

3. If quality checks fail:
   - Show the errors to the user
   - Do NOT commit
   - Explain what needs to be fixed

4. Never push to remote unless explicitly requested by the user
