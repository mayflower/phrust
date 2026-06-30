<?php
// runtime-semantics: category=callables expect=pass
class BinderCallableMatrix {
    public function join($first, $second = "B", ...$rest) {
        echo "method:", $first, "|", $second, "|", $rest["tail"], "\n";
    }

    public static function staticJoin($first, $second = "B", ...$rest) {
        echo "static:", $first, "|", $second, "|", $rest["tail"], "\n";
    }
}

$closure = function ($first, $second = "B", ...$rest) {
    echo "closure:", $first, "|", $second, "|", $rest["tail"], "\n";
};
$closure(second: "S", first: "F", tail: "T");

$object = new BinderCallableMatrix();
$object->join(second: "MS", first: "MF", tail: "MT");

$firstClass = $object->join(...);
$firstClass(second: "CS", first: "CF", tail: "CT");

call_user_func_array([BinderCallableMatrix::class, "staticJoin"], [
    "second" => "SS",
    "first" => "SF",
    "tail" => "ST",
]);
