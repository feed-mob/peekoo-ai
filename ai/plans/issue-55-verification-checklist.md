# Issue #55 Verification Checklist

**Scope**: Validate the OpenClaw-style first-run bootstrap and memory persistence flow added in PR #71.

## Preconditions

- Use a fresh local repo or a temp directory with no existing `.peekoo/`.
- Ensure the desktop app is built from the branch `feat/openclaw-bootstrap-memory`.
- If you want to validate global fallback instead of repo-local behavior, do not create a local `.peekoo/`.

## A. Fresh Workspace Bootstrap

1. Start with no `.peekoo/` directory in the repo root.
2. Launch the desktop app.
3. Confirm a `.peekoo/` directory is created automatically.
4. Confirm these files exist inside `.peekoo/`:
   - `AGENTS.md`
   - `BOOTSTRAP.md`
   - `IDENTITY.md`
   - `SOUL.md`
   - `USER.md`
   - `MEMORY.md`
   - `skills/memory-manager/SKILL.md`
5. Open `USER.md` and confirm the required fields are still `[NOT_SET]`.
6. Open chat and send a simple greeting.
7. Confirm Peekoo asks for missing profile information instead of acting like setup is already complete.

Expected result:

- Bootstrap files are seeded.
- The first response follows the onboarding behavior.

## B. Bootstrap Completion

1. Answer with a name and preferred form of address.
2. After the response completes, inspect `.peekoo/USER.md`.
3. Confirm `Name` and `Preferred address` are filled in with the provided values.
4. Confirm `.peekoo/BOOTSTRAP.md` has been removed.
5. Send another message in the same session.
6. Confirm Peekoo does not ask for onboarding information again.

Expected result:

- Required profile data is written to `USER.md`.
- `BOOTSTRAP.md` is deleted after setup completes.

## C. Durable Preference Memory

1. Tell Peekoo a durable preference, for example:
   - "Remember that I prefer short answers."
   - "Remember to call me Rich."
2. Finish the response and inspect `.peekoo/MEMORY.md` and `.peekoo/USER.md`.
3. Confirm the preference is stored in the correct file:
   - addressing/profile updates in `USER.md`
   - durable behavioral preferences in `MEMORY.md`
4. Confirm the skill did not dump raw chat transcripts or temporary notes into memory.

Expected result:

- Only durable facts are stored.
- The file choice matches the memory-manager rules.

## D. Later Session Recall

1. Close the chat session or restart the app.
2. Reopen the app and start a new conversation.
3. Ask a neutral follow-up such as:
   - "Do you remember my name?"
   - "How should you answer me?"
4. Confirm Peekoo uses the stored name/addressing preference correctly.
5. Confirm Peekoo does not ask for the initial onboarding information again.

Expected result:

- User identity and durable preferences persist across sessions.

## E. Session History Reference

1. Have a short conversation about a concrete topic in session 1.
2. Restart the app or open a later chat session.
3. Ask Peekoo about something from the recent prior conversation.
4. Confirm whether the answer comes from restored session context.
5. If recall is weak or absent, note whether the failure is:
   - session restore did not resume the expected conversation
   - durable memory was never written
   - the product requires explicit summary/retrieval beyond current behavior

Expected result:

- Recent history should be available through restored sessions.
- Failures here indicate the next missing implementation area.

## F. Repo-Local vs Global Workspace Selection

1. Create a local `.peekoo/` by running the app from the repo root.
2. Confirm files are created in the repo-local `.peekoo/`.
3. Remove the repo-local `.peekoo/`.
4. Launch the app again from a location without a local `.peekoo/`.
5. Confirm the app falls back to the global `.peekoo/` location.

Expected result:

- Repo-local workspace wins when present.
- Global workspace is used only as fallback.

## Pass Criteria

The change is ready for Issue #55 follow-up work if:

- bootstrap is seeded correctly
- `BOOTSTRAP.md` is removed after first-run setup
- `USER.md` and `MEMORY.md` are updated correctly
- later sessions remember user identity/preferences
- session-history behavior is understood well enough to decide whether more implementation is required

## Likely Follow-Up If Something Fails

- If `BOOTSTRAP.md` is never removed: improve or harden bootstrap completion logic.
- If memory is written to the wrong file: tighten `memory-manager` or `AGENTS.md` instructions.
- If history recall is weak across sessions: implement explicit summary or retrieval behavior.
- If repo-local/global selection is inconsistent: adjust workspace discovery in `workspace_bootstrap.rs`.
