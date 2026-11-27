# Pull Request

**By submitting this pull request, I confirm that:**
*(Tick each box that applies, e.g. `[x]`.)*

- [ ] I've read the [contributing guide](https://github.com/tronyx/rollcall/blob/develop/.github/CONTRIBUTING.md) and this whole template.
- [ ] This pull request makes one change.
- [ ] I've updated the README or `docs/` if needed.
- [ ] I've added a line about my change under **Unreleased** in `CHANGELOG.md`.
- [ ] I've tested my change and added or updated tests for it.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass in `backend/`.
- [ ] `npm run lint`, `npm run check`, `npm test` and `npm run build` pass in `frontend/`.
- [ ] If I changed a query or a migration, I ran `cargo sqlx prepare -- --all-targets` and committed `backend/.sqlx/`.
- [ ] I'm willing to help maintain this change if problems come up later.
- [ ] I'm giving this submission freely and claim no ownership of it.

---

## 🎯 What does this pull request do?

*Describe the problem or feature, with screenshots if useful and links to any related issues.*

## 🛠️ How does it do it?

*Summarize the changes, like a short changelog.*

## 📝 Documentation changes

*List any changes needed in the README, `docs/` or elsewhere, or write "None".*

---

> [!NOTE]
> Pull requests that don't follow this template will be closed. This project is maintained by one volunteer (me!) in my spare time, so thanks for your patience with replies.
