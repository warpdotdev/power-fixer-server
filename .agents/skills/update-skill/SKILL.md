---
name: update-skill
description: Creates or updates skills in this repository. Use when the user wants to add a new skill, modify an existing skill, or needs guidance on skill structure and best practices.
---

# Update Skill

This skill provides instructions for creating or updating skills in this repository. Skills are reusable instruction sets that teach agents how to perform specific tasks.

## When to use this skill

Use this skill when:
- Creating a new skill from scratch
- Updating or improving an existing skill
- Validating skill structure and format
- Getting guidance on skill best practices

## Skill file requirements

Every skill must be in its own directory under `.agents/skills/` with a `SKILL.md` file containing:

### Required YAML frontmatter
```yaml
---
name: skill-name
description: Brief description of what this skill does and when to use it
---
```

**Name requirements:**
- Kebab-case identifier (lowercase letters, numbers, hyphens only)
- Examples: `add-feature-flag`, `rust-unit-tests`, `update-skill`

**Description requirements:**
- Must be non-empty and specific
- Include key terms for skill discovery
- Begin with an action verb stating what the skill accomplishes
- Include when to use the skill
- Write in third person (e.g., "Adds feature flags..." not "I can help you add...")

**Good description examples:**
- `git-commit`: "Generate descriptive commit messages by analyzing git diffs. Use when the user asks for help writing commit messages or reviewing staged changes."
- `pdf-processing`: "Extract text and tables from PDF files, fill forms, merge documents. Use when working with PDF files or when the user mentions PDFs, forms, or document extraction."

For more details on writing effective descriptions, see [references/best-practices.md](references/best-practices.md).

## Typical skill structure

1. **Title and brief summary** - Clear title and concise overview of the skill's purpose and primary use cases
2. **Overview** - Context about the skill's purpose (optional but common)
3. **Main content** - Steps, usage instructions, or workflow guidance
4. **Best Practices** - Guidelines and recommendations (optional)
5. **Examples / Reference PRs** - Links to real examples (optional)

Keep structure flexible based on the skill's needs. Simple skills can omit optional sections.

## Creating a new skill

Follow these steps when creating a new skill:

1. **Choose a descriptive kebab-case name** for the skill directory
2. **Create the directory structure:**
   ```bash
   mkdir -p .agents/skills/skill-name/references
   ```
3. **Create SKILL.md** with proper YAML frontmatter
4. **Write clear, actionable instructions** in the main content
5. **Add supporting files** (if needed) in the `references/` subdirectory
6. **Validate the skill** structure and content

## When to split content

Keep skills concise. For complex skills (>200 lines):
- Keep essential workflow and procedural instructions in SKILL.md
- Move detailed reference material, schemas, and extensive examples to `references/` subdirectory
- Link to reference files using relative paths: `[references/best-practices.md](references/best-practices.md)`

Create a `references/` subdirectory when:
- SKILL.md approaches 200+ lines
- Skill covers multiple domains or workflows that can be loaded independently
- Detailed reference material would clutter the main instructions

## Supporting files

Skills can include supporting files like scripts, templates, or configuration files. Place these in the skill directory:

```
.agents/skills/
└── check-broken-links/
    ├── SKILL.md
    ├── check_links.py      # Supporting script
    └── config.yaml         # Configuration file
```

Reference these files in SKILL.md using relative paths from the project root.

## Validation

Optionally validate skills using the `skills-ref` tool:
```bash
skills-ref validate ./.agents/skills/skill-name
```

This checks YAML frontmatter and naming conventions. If not installed, use web search to learn about this package.

## Best practices summary

- **Write clear descriptions** - The description is how agents decide whether to use your skill
- **Be specific in instructions** - Include exact file paths, command syntax, and expected formats
- **Include examples** - Show concrete use cases to help agents understand intent
- **Keep skills focused** - Each skill should do one thing well
- **Use consistent naming** - Follow kebab-case convention
- **Version control your skills** - Commit skills to the repo so the whole team benefits

For detailed guidance on writing effective skills, see [references/best-practices.md](references/best-practices.md).

## Examples from this repository

After creating more skills, refer to them as examples:
- `.agents/skills/update-skill/SKILL.md` - This skill, demonstrating skill creation workflow
