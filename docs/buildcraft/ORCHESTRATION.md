# How BuildCraft work is run

Status: in use since Nov 10, 2026.

BuildCraft is built by one lead session and one worker at a time. The lead plans, hands out one
work order, and reviews what comes back. The worker builds exactly that order. The owner looks at
the result and decides what is next.

## Roles

| Role | Who | Does |
|---|---|---|
| Owner | The repository owner | Chooses priorities, tries the build, accepts or rejects |
| Lead | The most capable model available | Writes work orders, reviews finished work, does upstream merges and anything a worker failed at twice |
| Worker | A cheaper model, one at a time | Builds one work order on its own branch and reports what ran |
| Inspector | CI on the pull request | Runs format, lint and the whole test suite |

## The loop

1. The lead reads `STATUS.md` and picks the first phase that is not done.
2. If the phase has no work order in `work-orders/`, the lead writes one: the goal, the files to
   add or touch, the hooks allowed in upstream files, the acceptance list and the exact commands
   to run. A work order fits one sitting; a phase that does not fit is split.
3. The lead starts one worker with that work order. Workers run one after another, never side by
   side: the phases touch the same files and one machine builds one tree at a time.
4. The worker follows `ARCHITECTURE.md` §8, commits to a branch named in the work order, and
   reports what it changed, what it ran and what it could not run.
5. The lead reads the diff, runs the scoped tests again, and opens or updates the pull request.
   Red CI goes back to the worker once; a second failure is the lead's to fix.
6. Green CI: the lead updates `STATUS.md` and tells the owner what to try in the build.

## Rules for the lead

- Do not write feature code a worker can write. Spend the lead on plans, reviews and merges.
- A work order names real identifiers. Open the definition before naming it.
- Keep `STATUS.md` true: a phase is done when its acceptance list passed and CI was green, and
  the row says what is still missing.
- Upstream merges (`ARCHITECTURE.md` §3) are the lead's job, weekly, before new feature work.

## Rules for a worker

- Read `AGENTS.md`, `CODEBASE_MAP.md`, `PATCHES.md` and the work order. Do only the work order.
- New code goes in BuildCraft files. An upstream file gets a minimal hook and a `PATCHES.md` row
  in the same commit.
- Scope every cargo command to the crates touched, with `-j 2` and under `timeout`.
- Stop and report instead of widening scope when the order cannot be done as written.
