--TEST--
wp.core-language: function exit terminates script
--DESCRIPTION--
WordPress bootstrap functions call exit and die from nested control-flow; that
must terminate the whole request rather than acting like a local return.
--FILE--
<?php
function wp_like_exit_gate($flag, $message) {
    echo "gate|";
    if ($flag) {
        die((string) $message);
    }
    echo "after-gate|";
}

echo "start|";
wp_like_exit_gate(true, "stop");
echo "bad";
?>
--EXPECT--
start|gate|stop
