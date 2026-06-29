<?php
// runtime-semantics: category=wordpress_blockers expect=pass
$old = error_reporting(E_ALL & ~E_DEPRECATED);
echo (error_reporting() & E_DEPRECATED) === 0 ? "masked\n" : "visible\n";
error_reporting($old);
