<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_dispatch fixture_id=WP_A_INCLUDED_INHERITED_STATIC wp_area=static_dispatch_include
// Reduced WordPress language/VM fixture: inherited static methods work when the parent is loaded before call time.
require __DIR__ . "/_data/included-parent.php";

class IncludedChild extends IncludedParent
{
}

echo IncludedChild::label(), "\n";
