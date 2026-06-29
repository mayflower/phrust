<?php
// runtime-semantics: expect=known_gap known_gap=autoload_relation_dynamic_class_lookup
spl_autoload_register(function ($class) {
    include (__DIR__ . "/_data/AutoloadRelationCacheChild.php");
});

$object = new AutoloadRelationCacheChild();
echo ($object instanceof AutoloadRelationCacheBase) ? "autoload-relation=yes\n" : "autoload-relation=no\n";
