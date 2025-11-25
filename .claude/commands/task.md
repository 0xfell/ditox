# Create New Task

Create a new task file based on the user's description.

## Instructions

1. **Determine next task number**: Look at existing files in `docs/tasks/` (all subdirectories) and find the highest `NNN-` prefix, then increment by 1.

2. **Generate task file**: Create a new file at `docs/tasks/planned/NNN-slug.md` where:
   - `NNN` is the next number (zero-padded to 3 digits)
   - `slug` is a short kebab-case name derived from the description

3. **Fill in the template** based on the user's description:
   - Title
   - Description (expanded from user input)
   - Requirements as checklist items
   - Priority (ask if unclear, default to medium)
   - Created date (today)

4. **Update ROADMAP.md**: Add the new task to the "Planned" table in `docs/ROADMAP.md` and increment the Planned count.

5. **Report**: Show the user the created task file path and a summary.

## User's Task Description

$ARGUMENTS
