# Commit Command

Add and commit changes with a meaningful commit message.

## Instructions

When this command is invoked:

1. **Review Changes**
   - Run `git status` to see modified files
   - Run `git diff` to see actual changes
   - Review recent commit history with `git log` to match style

2. **Analyze Changes**
   - Identify the nature of changes (feature, fix, refactor, docs, etc.)
   - Determine scope (which module/area affected)
   - Understand the purpose and impact

3. **Create Meaningful Commit Message**
   - Start with a verb (Add, Fix, Update, Refactor, Remove, etc.)
   - Be specific about what changed
   - Focus on "why" not just "what"
   - Keep first line under 50 characters if possible
   - Add details in body if needed

4. **Commit Process**
   - Add relevant files with `git add`
   - Create commit with descriptive message
   - Verify with `git status`

## Message Format

```
<verb> <what changed>

Optional: More detailed explanation of why this change
was made, what problem it solves, or context needed.
```

## Examples

Good commit messages:

- `Add pause menu with overlay and controls`
- `Fix player corner-sticking with multi-point collision`
- `Refactor audio system to use HashMap for sounds`
- `Update CLAUDE.md with v0.2.0 audio architecture`
- `Remove unused debug logging from ghost AI`

Bad commit messages:

- `changes`
- `fix`
- `wip`
- `update stuff`
