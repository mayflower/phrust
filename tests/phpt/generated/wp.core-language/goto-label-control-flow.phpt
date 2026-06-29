--TEST--
wp.core-language: goto label control flow
--DESCRIPTION--
WordPress UTF-8 bootstrap helpers use forward goto labels to leave nested
control-flow without treating the skipped statements as executed.
--FILE--
<?php
function wp_like_scan($limit) {
    for ($i = 0; $i < 4; $i++) {
        if ($i === $limit) {
            goto found;
        }
        echo "scan:$i|";
    }

    found:
    echo "found:$i\n";
    return $i;
}

function wp_like_fallthrough($flag) {
    if ($flag) {
        goto done;
    }
    echo "fallthrough|";

    done:
    return "done";
}

echo "ret=", wp_like_scan(2), "\n";
echo wp_like_fallthrough(true), "\n";
echo wp_like_fallthrough(false), "\n";
?>
--EXPECT--
ret=scan:0|scan:1|found:2
2
done
fallthrough|done
