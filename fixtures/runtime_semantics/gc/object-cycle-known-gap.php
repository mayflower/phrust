<?php
// runtime-semantics: category=gc expect=known_gap known_gap=E_PHP_RUNTIME_GC_PUBLIC_API_GAP
class Node {
    public ?Node $next = null;
}

$node = new Node();
$node->next = $node;
unset($node);
echo gc_collect_cycles(), "\n";
