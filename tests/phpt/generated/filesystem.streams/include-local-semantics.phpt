--TEST--
filesystem.streams: include local semantics
--DESCRIPTION--
Generated include baseline covering include return values, shared top-level
scope, include_once, and include_path local search.
--FILE--
<?php
$dir = __DIR__ . "/include-local-semantics-lib";
@mkdir($dir);
$child = $dir . "/child.php";
$once = $dir . "/once.php";
$path = $dir . "/path.php";
$direct = $dir . "/direct.php";
file_put_contents($child, '<?php $shared = $shared . "|child"; echo "child|"; return 11; ?>');
file_put_contents($once, '<?php echo "once|"; return 5; ?>');
file_put_contents($path, '<?php echo "path|"; return 7; ?>');
file_put_contents($direct, '<?php echo "direct|"; return 13; ?>');
$shared = "parent";
$value = include $child;
echo "value=$value|shared=$shared\n";
$directValue = require __DIR__ . "/include-local-semantics-lib/direct.php";
echo "direct-value=$directValue\n";
$first = include_once $once;
$second = include_once $once;
echo "once-values=$first:$second\n";
ini_set("include_path", $dir);
$pathValue = include "path.php";
echo "path-value=$pathValue\n";
?>
--CLEAN--
<?php
$dir = __DIR__ . "/include-local-semantics-lib";
@unlink($dir . "/child.php");
@unlink($dir . "/once.php");
@unlink($dir . "/path.php");
@unlink($dir . "/direct.php");
@rmdir($dir);
?>
--EXPECT--
child|value=11|shared=parent|child
direct|direct-value=13
once|once-values=5:1
path|path-value=7
