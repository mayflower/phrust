<?php
// Returns unwind through every enclosing finally, innermost first, and a
// finally can override the pending return. Also pins that a moved return
// value keeps identity (no defensive clone on the way out).
function with_finally() {
    $arr = ['a' => 1];
    try {
        return $arr;
    } finally {
        echo "F";
    }
}
function nested() {
    try {
        try {
            return "inner";
        } finally {
            echo "1";
        }
    } finally {
        echo "2";
    }
}
function finally_overrides() {
    try {
        return "try";
    } finally {
        return "finally";
    }
}
$r = with_finally();
echo ":", $r['a'], "\n";
echo nested(), "\n";
echo finally_overrides(), "\n";
$big = ['x' => str_repeat('y', 10)];
function plain($v) { return $v; }
var_dump(plain($big) === $big);
