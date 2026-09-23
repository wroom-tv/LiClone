# Changelog

## 1.0.0

First release of LiClone for Windows.

### Setup

- First launch asks you to install rclone if it is missing, and to start LiClone when you sign in. Starting with Windows is on unless you turn it off.
- When that option is on, LiClone opens in the tray.
- rclone is installed for your user and added to your PATH. LiClone tells you when the installed rclone is older than 1.65.

### Window

- The window has its own title bar. Closing it hides LiClone in the tray and shows a notification that the app is still running.
- The tray icon opens the window again. **Quit** on the tray icon exits.

### Mounts

- Save a mount profile and start it from LiClone, or start it when you sign in to Windows.
- Presets for everyday use, video, editing, browse-only, and low disk use. The preset that matches your settings stays highlighted.
- Each setting has a short explanation. Sliders and choices show **Default**, **No limit**, or **Off** instead of a blank field. Extra flags are there for any rclone argument the form does not list.
- A new mount suggests a cache size: 10% of free space on your profile disk, at least 1 GB, and never more than 100 GB.
- Each mount gets its own remote-control port. If 5572 or the saved port is already taken, LiClone picks the next free one and shows it when you save.
- LiClone refuses to start a second mount on the same drive letter, or on the same remote and folder, while that mount is already running.
- **Add to Mounts** copies a running rclone mount into a profile. If that drive letter is already saved, the same action updates that profile.
- Saved profiles can be edited again.

### Transfers and cache

- Transfers lists files that are still on this PC and have not finished uploading: saving to disk, waiting, or uploading.
- Clear cache shows a preview and waits for you to confirm. You can clear read cache that is not an upload, cached files whose size already matches the cloud, or read cache that has not been touched for a number of days. Files that are still uploading are left alone.

### Remotes

- Create and edit every rclone remote, including fields rclone marks as advanced.
- Leave a password or token blank to keep the saved one. Reconnect starts browser sign-in again.
- Remote passwords stay in rclone’s config.

### Installer

- The Windows installer is published for your user account. It shows the Wroom license, uses the LiClone window art, and adds a Start menu shortcut under Wroom. An older installer will not replace a newer LiClone.

### License

- Licensed under the [Wroom Source-Available License 1.0](LICENSE). The project page is [wroom-tv/LiClone](https://github.com/wroom-tv/LiClone).
