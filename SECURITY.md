# Security

LiClone runs on your PC. It talks to rclone on this computer. It does not send your remotes, passwords, or file list to Wroom.

## Reporting a problem

If you find a security issue, open a private security advisory on [wroom-tv/LiClone](https://github.com/wroom-tv/LiClone/security/advisories/new). Do not open a public issue for it.

Say what you did, what you expected, and what happened. Include your Windows version and your rclone version.

Do not include:

- your rclone config
- passwords, tokens, or client secrets
- file names you do not want public
- a proof that would let someone else read another person’s files

We will reply when we have read the report. This license does not promise a fix or a timeline. See [LICENSE](LICENSE), section 13.

## What stays on this PC

- Mount profiles and the first-run choice live in your user data folder, under `liclone`.
- rclone keeps remote passwords and tokens in its own config. LiClone reads that config through rclone. It does not store a second copy of those secrets.
- Cache files stay where rclone put them, usually under your local AppData `rclone` folder.

## Remote control

A mount with remote control listens on `127.0.0.1` only, on the port shown when you save the profile. LiClone does not put a password on that port, because it is not reachable from other computers unless something else on this PC forwards it.

Do not publish that port, and do not point a tunnel at it. Anyone who can open it can ask that rclone process about the mount.

Two mounts never share a port. LiClone moves a profile to a free port when the one it had is already in use.

## Cache delete

**Clear cache** deletes local cache files after you confirm the preview. It does not delete the copy in the cloud. Read the preview before you confirm. Files rclone still has marked as uploads are skipped.

## Third-party software

rclone and WinFSP are not part of LiClone. Their security reports go to those projects.
