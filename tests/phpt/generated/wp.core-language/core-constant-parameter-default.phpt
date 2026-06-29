--TEST--
wp.core-language: core constants in parameter defaults
--DESCRIPTION--
WordPress bootstrap helpers use PHP integer constants as typed parameter
defaults, including PHP_INT_MAX in nullable integer parameters.
--FILE--
<?php
function wp_like_max_default(?int $value = PHP_INT_MAX): void {
    echo $value, "\n";
}

function wp_like_min_default(?int $value = PHP_INT_MIN): void {
    echo $value, "\n";
}

function wp_like_error_level(int $level = E_USER_NOTICE): void {
    echo $level, "\n";
}

wp_like_max_default();
wp_like_max_default(5);
wp_like_min_default();
wp_like_error_level();
?>
--EXPECT--
9223372036854775807
5
-9223372036854775808
1024
