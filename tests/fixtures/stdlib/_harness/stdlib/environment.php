<?php
// stdlib-diff: id=STDLIB_ENVIRONMENT area=stdlib expect=pass
putenv('PHRUST_STDLIB_ENV=controlled');
echo getenv('PHRUST_STDLIB_ENV'), "\n";
echo getenv('PHRUST_STDLIB_MISSING') === false ? "missing\n" : "bad\n";
$env = getenv();
echo isset($env['PHRUST_STDLIB_ENV']) ? "array-env\n" : "bad\n";
putenv('PHRUST_STDLIB_ENV');
echo getenv('PHRUST_STDLIB_ENV') === false ? "unset\n" : "bad\n";
if (php_sapi_name() === 'cli') { echo "cli\n"; } else { echo "bad\n"; }
if (is_string(php_uname())) { echo "uname\n"; } else { echo "bad\n"; }
if (is_string(php_uname('s'))) { echo "uname-s\n"; } else { echo "bad\n"; }
if (is_string(get_current_user())) { echo "user\n"; } else { echo "bad\n"; }
if (isset($_SERVER['argc']) && isset($_SERVER['argv'])) { echo "server-argv\n"; } else { echo "bad\n"; }
if (is_array($_ENV)) { echo "env-array\n"; } else { echo "bad\n"; }
