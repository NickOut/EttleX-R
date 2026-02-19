# Cleanup Command

Review the codebase for any leftover code, unnecessary complexity, or simplification opportunities.

## Tasks

1. **Remove Dead Code**
   - Unused imports
   - Commented-out code blocks
   - Unused functions or variables
   - Unreachable code paths

2. **Simplify Complex Code**
   - Overly complex conditionals
   - Redundant logic
   - Opportunities for clearer code structure
   - Unnecessary abstractions

3. **Code Quality**
   - Ensure consistent formatting
   - Check for TODO/FIXME comments
   - Verify documentation is accurate
   - Remove debug prints or logging

4. **Architecture Review**
   - Check for violations of separation of concerns
   - Ensure modules follow documented responsibilities
   - Verify constants are used instead of magic numbers

## Process

1. Run `make check` to check for formatting and linting issues
2. Review each module systematically
3. Make targeted improvements
4. Run tests after each change to ensure nothing breaks
5. Document any significant changes
