# Update stops

LiClone installs the nearest newer release it is allowed to take. It reads `updates.json` from the latest GitHub release. That file is the list in this repository.

Each step has:

- **version** — the release to install
- **min** — the oldest installed version that may install it
- **max** — optional newest installed version that should still stop here

Someone on 1.2.0 does not jump to 2.7.0 when 2.7.0 says its oldest readable settings start at 1.9.0. They install 1.9.0 first. After restart, 1.9.0 is new enough, so the next open installs 2.7.0.

```json
{
  "route": [
    { "version": "1.9.0", "min": "1.2.0", "max": "1.8.9" },
    { "version": "2.7.0", "min": "1.9.0" }
  ]
}
```

When you publish a release, set the last step to that version and attach `updates.json` to the release. The installers for each step stay on their own release, signed with the same key.
