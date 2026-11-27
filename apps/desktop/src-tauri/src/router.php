<?php
// Website Manager preview router.
//
// PHP's built-in server ignores .htaccess files. This router applies the access rules that
// matter for the preview – "Require all denied|granted" and "Deny/Allow from all", also inside
// <FilesMatch> and <IfModule> – and blocks hidden files. It protects sample data such as user
// files during the local preview; it is not a replacement for the web server's rules.

$root = rtrim(str_replace('\\', '/', (string) realpath($_SERVER['DOCUMENT_ROOT'])), '/');
$path = rawurldecode((string) (parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH) ?? '/'));

if (preg_match('#(^|/)\.(?!well-known(/|$))#', $path)) {
    deny();
}

$target = realpath($root . $path);
if ($target === false) {
    return false;
}
$target = str_replace('\\', '/', $target);
if ($target !== $root && !str_starts_with($target, $root . '/')) {
    deny();
}

$isDirectory = is_dir($target);
$directory = $isDirectory ? $target : dirname($target);
$file = $isDirectory ? '' : basename($target);

$chain = [];
for ($current = $directory; ; $current = dirname($current)) {
    $chain[] = $current;
    if ($current === $root || strlen($current) <= strlen($root)) {
        break;
    }
}

$allowed = true;
foreach (array_reverse($chain) as $current) {
    $rules = @file_get_contents($current . '/.htaccess');
    if ($rules !== false) {
        $allowed = apply_rules($rules, $file, $allowed);
    }
}
if (!$allowed) {
    deny();
}
return false;

function deny(): never
{
    http_response_code(403);
    header('Content-Type: text/plain; charset=utf-8');
    echo "403 Forbidden (preview)\n";
    exit;
}

function apply_rules(string $rules, string $file, bool $allowed): bool
{
    $stack = [];
    $directoryDecision = null;
    $fileDecision = null;
    foreach (preg_split('/\R/', $rules) ?: [] as $line) {
        $line = trim($line);
        if ($line === '' || $line[0] === '#') {
            continue;
        }
        if (preg_match('#^<IfModule\s+(!)?#i', $line, $match)) {
            $stack[] = ['files' => false, 'active' => empty($match[1])];
            continue;
        }
        if (preg_match('#^<FilesMatch\s+"([^"]+)"#i', $line, $match)) {
            $pattern = '#' . str_replace('#', '\#', $match[1]) . '#';
            $stack[] = ['files' => true, 'active' => $file !== '' && @preg_match($pattern, $file) === 1];
            continue;
        }
        if (preg_match('#^</(IfModule|FilesMatch)#i', $line)) {
            array_pop($stack);
            continue;
        }
        if (!preg_match('#^(Require\s+all\s+(denied|granted)|(Deny|Allow)\s+from\s+all)#i', $line, $match)) {
            continue;
        }
        foreach ($stack as $entry) {
            if (!$entry['active']) {
                continue 2;
            }
        }
        $grant = stripos($line, 'granted') !== false || stripos($line, 'allow') === 0;
        $insideFiles = array_filter($stack, static fn (array $entry): bool => $entry['files']) !== [];
        if ($insideFiles) {
            $fileDecision = $grant;
        } else {
            $directoryDecision = $grant;
        }
    }
    return $fileDecision ?? $directoryDecision ?? $allowed;
}
