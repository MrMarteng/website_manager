# Third-party programs

Release builds of the Website Manager ship these programs unchanged, next to the app. Versions,
download URLs and SHA-256 checksums are pinned in `apps/desktop/sidecars.json`.

| Program | Version | Source | License |
|---|---|---|---|
| PHP (static build, CLI) | 8.5.8, Windows 8.5.5 | [static-php-cli](https://github.com/crazywhalecc/static-php-cli) prebuilt binaries (`dl.static-php.dev`) | PHP License 3.01, see `PHP-LICENSE.txt` |
| rclone | 1.76.0 | [rclone.org](https://rclone.org/downloads/) | MIT, see `rclone-COPYING.txt` |

The static PHP build links further open-source libraries (for example OpenSSL, libxml2, libgd,
libpng, libjpeg, zlib, SQLite, oniguruma, curl). Their licenses apply to those parts; they are
listed in the static-php-cli documentation of the respective release.
