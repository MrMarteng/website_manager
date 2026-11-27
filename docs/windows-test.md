# Testing on Windows

For Windows 10 and 11 (x64). Two ways to get the program: download the installer from GitHub
Actions (recommended) or build it yourself.

## 1. Get the program

### a) Installer from GitHub Actions

1. On GitHub: **Actions → Build → Run workflow** (or push a tag `v*`).
2. After the run, download and unpack `website-manager-Windows-<number>` from **Artifacts**. It
   contains the installer (`*.msi` or `*-setup.exe`) and the command line program
   `website-manager.exe`.
3. Run the installer. The installers are not signed yet; SmartScreen reports "Windows protected
   your PC" → **More info → Run anyway**.

### b) Build it yourself

Install once:

* [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/),
  workload "Desktop development with C++"
* Rust via [rustup](https://rustup.rs) (`rustup-init.exe`, default settings)
* [Node.js LTS](https://nodejs.org) (only for the Tauri CLI) and [Git](https://git-scm.com)
* WebView2 is part of Windows 10/11.

Then in PowerShell:

```powershell
git clone https://github.com/MrMarteng/website_manager.git website-manager
cd website-manager\apps\desktop
npm ci
npm run dev      # start the program in development mode
npm run build    # build the installer → ..\..\target\release\bundle\msi and \nsis
```

With `npm run dev` the `.tsweb` file extension is not registered; test double-clicking a package
with an installed build only.

## 2. Provide PHP and rclone

The app does not ship either of them yet. The app itself is in German or English; the steps below
use the English Windows names.

1. **PHP 8.2 or newer:** unpack the ZIP "VS17 x64 Non Thread Safe" from
   [windows.php.net](https://windows.php.net/download/) to `C:\tools\php`, copy
   `php.ini-production` to `php.ini` and enable `extension_dir = "ext"`, `extension=mbstring`,
   `extension=fileinfo` and `extension=gd` in it (remove the semicolon at the start of the line;
   `dom` and `json` are built in).
2. **rclone:** unpack the ZIP "Windows amd64" from [rclone.org](https://rclone.org/downloads/) to
   `C:\tools\rclone`.
3. Set user environment variables (Settings → System → About → Advanced system settings →
   Environment Variables) and restart the program afterwards:

   | Variable | Value |
   |---|---|
   | `WM_PHP` | `C:\tools\php\php.exe` |
   | `WM_RCLONE` | `C:\tools\rclone\rclone.exe` |

Check in a new PowerShell: `& $env:WM_PHP -v` and `& $env:WM_RCLONE version`.

## 3. Test package

Packages are built on the agency's computer (`website-manager pack`, see README) and brought to
the Windows computer by USB stick, network drive or mail. A first delivery with credentials and an
update of the same project are good test cases.

## 4. Checklist

UI labels are given in English; in the German UI they read accordingly.

| # | Check | Expected |
|---|---|---|
| 1 | Start the program | window appears, language follows Windows (German/English) |
| 2 | Open a package with the button | overview with version, publisher, changes |
| 3 | Open a package by drag and drop and by double-click (installed build) | opens; with the program running, in the same window |
| 4 | Package from another publisher (other key, same name) | rejected |
| 5 | Take over access (package password) | **Credential Manager → Windows Credentials** lists `com.teleschirm.website-manager` |
| 6 | View in browser | website in the default browser, **no** firewall prompt; editing system login loads |
| 7 | Stop preview, close the program | no `php.exe` left in Task Manager |
| 8 | Check changes / upload into a test folder (for example `C:\temp\server`) | list of new and deleted files, files in the folder, `.website-manager.json` present |
| 9 | Open an update | settings and access are kept, old version under "Versions" |
| 10 | Restore a version and upload it | question about the newer version on the target |
| 11 | Rebuild | downloads Node.js the first time, reports "Rebuilt"; preview shows the new build |
| 12 | Start and stop development mode | page opens in the browser; afterwards no `node.exe`/`php.exe` in Task Manager |
| 13 | User name or folder with spaces/umlauts | everything as above |
| 14 | Save the log as a file | text file with the run |

## 5. Where the data is

| What | Location |
|---|---|
| projects, versions, `trust.json` | `%APPDATA%\com.teleschirm.website-manager\` |
| Node.js cache | `%LOCALAPPDATA%\com.teleschirm.website-manager\node\` |
| access | Windows Credential Manager |

To reset, close the program, delete both folders and remove the entry in Credential Manager.

## 6. Reporting errors

In the program, **"For experts: log" → "Save as file …"**, and attach the file with the number from
the checklist to the issue. For crashes, also start `npm run dev` from PowerShell and send the
terminal output.
