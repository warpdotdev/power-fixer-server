# Skill Authoring Best Practices

This document provides detailed guidance on writing effective skills. These best practices help ensure skills are discoverable, usable, and maintainable.

## Description Best Practices

The description field in YAML frontmatter is critical for skill discovery. Agents use descriptions to decide whether a skill is relevant to the user's request.

### Effective descriptions

**Structure:** Start with an action verb + what it does + when to use it

Good examples:
- "Generate descriptive commit messages by analyzing git diffs. Use when the user asks for help writing commit messages or reviewing staged changes."
- "Extract text and tables from PDF files, fill forms, merge documents. Use when working with PDF files or when the user mentions PDFs, forms, or document extraction."
- "Add feature flags with proper configuration and documentation. Use when creating new feature toggles or enabling conditional functionality."

### What makes a good description

1. **Action-oriented** - Begin with a verb that describes the primary action (Generate, Extract, Add, Deploy, etc.)
2. **Specific** - Include concrete details about what the skill does, not vague statements
3. **Context-rich** - Include key terms and domain-specific words that users might mention
4. **When-clause** - Explicitly state when to use the skill (e.g., "Use when...")
5. **Third person** - Write objectively (e.g., "Adds..." not "I add...")

### Common description anti-patterns

❌ **Too vague:**
- "Helps with code" - What kind of code? What kind of help?
- "Does development tasks" - Too broad, no specific use case

❌ **Missing when-clause:**
- "Creates database migrations" - Missing context about when to use this

❌ **First person:**
- "I can help you deploy" - Use third person instead

❌ **Too narrow:**
- "Deploys version 2.3.1 to production" - Too specific to be reusable

✅ **Good:**
- "Creates and runs database migrations. Use when schema changes need to be applied to the database."

## Conciseness Principles

Skills should be concise and focused. Agents process skills as part of their context, so brevity improves performance and comprehension.

### Progressive disclosure

Structure information from essential to detailed:

1. **Start with the most important information** - What the skill does and when to use it
2. **Provide workflow steps** - Clear, actionable instructions
3. **Defer details to references** - Move extensive documentation to `references/` subdirectory
4. **Link to references** - Use relative paths so agents can load details when needed

Example structure:
```markdown
# Skill Name
Brief overview and when to use it.

## Main workflow
1. Step one
2. Step two
3. Step three

For detailed configuration options, see [references/configuration.md](references/configuration.md).
```

### Keep SKILL.md focused

- **Aim for < 200 lines** in SKILL.md
- **Extract detailed content** to reference files when skills grow
- **Avoid walls of text** - Use bullet points, numbered lists, and headings
- **Remove redundant explanations** - Trust that agents understand basic concepts

### What belongs in SKILL.md vs references/

**SKILL.md should contain:**
- Essential workflow steps
- Common use cases
- Quick reference for key commands or patterns
- Links to reference files

**references/ should contain:**
- Detailed configuration schemas
- Extensive code examples
- Background information and context
- Troubleshooting guides
- API documentation

## Code Example Formatting

Code examples help agents understand expected patterns and syntax. Format them clearly and consistently.

### Inline code

Use backticks for:
- File paths: `.agents/skills/skill-name/SKILL.md`
- Command names: `cargo build`
- Variable names: `DATABASE_URL`
- Short code snippets: `let x = 5;`

### Code blocks

Use fenced code blocks with language identifiers:

````markdown
```yaml
---
name: skill-name
description: Description here
---
```

```bash
mkdir -p .agents/skills/skill-name
```

```rust
fn main() {
    println!("Hello, world!");
}
```
````

### Command examples

For shell commands, show the full command with context:

✅ **Good:**
```bash
# Create skill directory
mkdir -p .agents/skills/deploy-production

# Validate the skill
skills-ref validate ./.agents/skills/deploy-production
```

❌ **Avoid:**
```
mkdir skill-dir
validate skill
```

### When to include examples

Include code examples when:
- The syntax is not obvious
- There are multiple valid approaches
- The example clarifies a complex concept
- You're showing a specific file format or structure

## File Organization

### When to create a references/ subdirectory

Create `references/` when:
- SKILL.md exceeds 200 lines
- You have extensive documentation that would clutter the main workflow
- The skill covers multiple domains that can be loaded independently
- You need to store schemas, templates, or detailed technical specs

### Naming reference files

Use descriptive names that indicate content:
- `references/best-practices.md` - Best practices guide
- `references/configuration.md` - Configuration options
- `references/api-reference.md` - API documentation
- `references/troubleshooting.md` - Common issues and solutions

### Linking to reference files

Always use relative paths from SKILL.md:
```markdown
For detailed configuration options, see [references/configuration.md](references/configuration.md).
```

Avoid absolute paths or external URLs for internal documentation.

## Writing Style

### Use clear, direct language

✅ **Good:**
- "Create a new branch for the feature"
- "Run the test suite to verify changes"

❌ **Avoid:**
- "You should probably create a new branch"
- "It might be a good idea to run tests"

### Use imperative mood for instructions

✅ **Good:**
- "Clone the repository"
- "Install dependencies"
- "Deploy to production"

❌ **Avoid:**
- "You can clone the repository"
- "Dependencies should be installed"
- "The application needs to be deployed"

### Be specific about paths and commands

✅ **Good:**
```bash
# Install dependencies from the project root
npm install --prefix ./frontend
```

❌ **Avoid:**
```bash
# Install dependencies
npm install
```

## Common Anti-Patterns

### 1. Skills that are too broad

❌ **Avoid:**
```yaml
name: development
description: Helps with development tasks
```

✅ **Instead, create focused skills:**
```yaml
name: add-api-endpoint
description: Add a new REST API endpoint with tests and documentation
```

### 2. Missing when-clause in description

❌ **Avoid:**
```yaml
description: Deploys the application to production
```

✅ **Include when to use it:**
```yaml
description: Deploys the application to production. Use when releasing new versions or hotfixes to the production environment
```

### 3. Instructions without context

❌ **Avoid:**
```markdown
1. Run the script
2. Check the output
3. Deploy
```

✅ **Provide context:**
```markdown
1. Run the deployment script: `./script/deploy.sh --env production`
2. Verify the health check endpoint: `curl https://api.example.com/health`
3. Monitor logs for errors: `gcloud logging read --project prod`
```

### 4. Mixing workflows

❌ **Avoid:**
A single skill that handles both deployment AND rollback AND monitoring

✅ **Split into focused skills:**
- `deploy-production` - Deploy to production
- `rollback-deployment` - Rollback a failed deployment
- `monitor-production` - Monitor production health

### 5. Assuming too much context

❌ **Avoid:**
"Run the usual checks before deploying"

✅ **Be explicit:**
"Run these checks before deploying:
1. `cargo test` - Run test suite
2. `cargo clippy` - Check for lints
3. `./script/integration-test.sh` - Run integration tests"

## Validation Checklist

Before committing a skill, verify:

- [ ] YAML frontmatter is valid with `name` and `description`
- [ ] Name is kebab-case (lowercase, hyphens only)
- [ ] Description includes both what and when
- [ ] Description uses third person and action verbs
- [ ] Instructions are clear and actionable
- [ ] File paths use relative paths
- [ ] Code examples include language identifiers
- [ ] Links to reference files work correctly
- [ ] SKILL.md is under 200 lines (or content is split appropriately)
- [ ] Supporting files are referenced in SKILL.md

## Examples of Well-Structured Skills

### Simple skill (< 100 lines)

```markdown
---
name: format-code
description: Format code using project-specific formatters. Use when code needs formatting or before committing changes
---

# Format Code

## When to use
Use this skill when code needs formatting or before committing changes.

## Instructions
1. Run the appropriate formatter for the language:
   - Rust: `cargo fmt`
   - Python: `black .`
   - JavaScript/TypeScript: `npm run format`
2. Verify no unexpected changes were made
3. Stage and commit the formatted code
```

### Complex skill with references

```markdown
---
name: database-migration
description: Create and apply database migrations safely. Use when schema changes are needed or during deployments
---

# Database Migration

## When to use
Use when making schema changes to the database or applying migrations during deployment.

## Quick workflow
1. Create migration: `./script/create-migration.sh migration_name`
2. Review the generated SQL in `migrations/`
3. Test locally: `./script/migrate.sh --local`
4. Apply to staging: `./script/migrate.sh --staging`
5. Apply to production: `./script/migrate.sh --production`

For detailed migration patterns and rollback procedures, see [references/migration-guide.md](references/migration-guide.md).
```

## Summary

Effective skills are:
- **Discoverable** - Clear descriptions with action verbs and when-clauses
- **Focused** - One skill does one thing well
- **Concise** - Keep SKILL.md under 200 lines, defer details to references/
- **Actionable** - Provide clear, specific instructions
- **Well-organized** - Use progressive disclosure and logical structure
- **Validated** - Follow naming conventions and include all required fields
