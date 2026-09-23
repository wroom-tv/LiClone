# LiClone

LiClone is a Windows app for [rclone](https://rclone.org/). It mounts cloud storage as a drive, shows files that still need to upload, and lets you edit remotes without a terminal.

You can use it for personal, work, or commercial use. You can read the source and change it for yourself. Publishing a copy or a changed version needs written permission from Wroom. The full terms are in [LICENSE](LICENSE).

## Install

Download the Windows installer from [Releases](https://github.com/wroom-tv/LiClone/releases). The installer is for the current user, shows the LiClone license before it copies files, and puts a shortcut under **Wroom** in the Start menu. It will not install an older version over a newer one.

You need Windows 10 or later. A drive letter also needs [WinFSP](https://winfsp.dev/). rclone and WinFSP are separate projects with their own licenses.

rclone does not have to be installed first. The first time LiClone opens, it walks you through three steps:

1. A short welcome.
2. **Install rclone**, if it is not already on this PC. That installs rclone for your user and adds it to your PATH.
3. **Start LiClone when I sign in.** This is on by default. LiClone then opens in the tray, without a window in the way. You can turn it off and continue.

LiClone is built for rclone 1.65 and newer. If the header says the installed rclone is too old, update rclone before you rely on mounts or remote edits.

## What you can do

- See files that are on this PC and have not finished uploading
- Save mount profiles, with presets for everyday use, video, editing, browse-only, and low disk use
- Give each mount its own remote-control port, so two mounts can run at the same time
- Add a mount that is already running, or edit a saved one
- Create and edit rclone remotes, including settings rclone marks as advanced
- Clear cache only after a preview, and never while a file is still uploading
- Close the window and leave LiClone in the tray. **Quit** in the tray menu is what exits

A walkthrough of each screen is in [Using LiClone](docs/usage.md).

## If something goes wrong

Report a security problem the way [SECURITY.md](SECURITY.md) describes. Do not paste passwords, tokens, or your rclone config into a public issue.

Other bugs and ideas go to [Issues](https://github.com/wroom-tv/LiClone/issues). If you want to send a fix, read [CONTRIBUTING.md](CONTRIBUTING.md) first.

## Build it yourself

You need Node 22 and a stable Rust toolchain.

```bash
npm ci
npm run tauri dev
```

`npm run tauri build` writes the installer to `src-tauri/target/release/bundle`.

## License

[Wroom Source-Available License 1.0](LICENSE). Copyright (c) 2026 Wroom. All rights reserved.
