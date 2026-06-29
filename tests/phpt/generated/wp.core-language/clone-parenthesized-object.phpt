--TEST--
wp.core-language: parenthesized clone operand
--DESCRIPTION--
WordPress bootstrap classes clone objects through a parenthesized operand.
--FILE--
<?php
class WpLikeCloneTarget {
    public $value = "source";
}

$source = new WpLikeCloneTarget;
$copy = clone($source);
$copy->value = "copy";

echo get_class($copy), "\n";
echo $source->value, "|", $copy->value, "\n";
?>
--EXPECT--
WpLikeCloneTarget
source|copy
