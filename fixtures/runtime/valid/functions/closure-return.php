<?php
function make_closure($x) {
    return function () use ($x) {
        return $x;
    };
}

$f = make_closure(9);
echo $f(), "\n";
