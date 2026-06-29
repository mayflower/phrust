--TEST--
wp.core-language: zero-argument die terminates script
--DESCRIPTION--
WordPress request handlers use bare die() in helper functions. It must lower as
an exit construct without a placeholder operand and terminate the request.
--FILE--
<?php
function wp_like_bare_die($flag) {
    echo "before|";
    if ($flag) {
        die();
    }
    echo "bad";
}

echo "start|";
wp_like_bare_die(true);
echo "bad";
?>
--EXPECT--
start|before|
