<?php
$base = getcwd();
$dir = $base . "/cwd-fixture";
if (!is_dir($dir)) {
    mkdir($dir);
}
file_put_contents($dir . "/value.txt", "from-cwd");
echo "changed=", chdir($dir) ? "yes" : "no", "\n";
echo "base-restored=", chdir($base) ? "yes" : "no", "\n";
chdir($dir);
echo "content=", file_get_contents("value.txt"), "\n";
chdir($base);
