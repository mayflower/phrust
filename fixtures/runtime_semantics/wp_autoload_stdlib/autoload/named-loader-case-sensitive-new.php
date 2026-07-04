<?php
// runtime-semantics: category=wp_autoload_stdlib expect=pass
// The autoloader receives the source-case class name even when the new
// expression executes on the dense plan (no closures here, so the whole
// unit lowers dense): a case-sensitive loader comparison must match.
function pack_b_case_sensitive_loader($class) {
    if ($class === "PackBExists") {
        include __DIR__ . "/_data/PackBExists.php";
    }
}
spl_autoload_register('pack_b_case_sensitive_loader');

$object = new PackBExists();
echo get_class($object), "\n";
