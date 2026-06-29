--TEST--
wp.core-language: nested inline HTML inside PHP blocks
--DESCRIPTION--
WordPress installer and admin templates leave PHP mode inside braced control
blocks and then resume PHP mode before the block closes.
--FILE--
<?php
if (false) { ?>hidden<?php }
if (true) { ?><span>shown</span><?php echo "|php|"; ?>tail<?php }
echo "\nend\n";
?>
--EXPECT--
<span>shown</span>|php|tail
end
