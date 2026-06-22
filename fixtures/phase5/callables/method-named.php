<?php
class CallBox {
    public function join($first, $second, ...$rest) {
        echo $first, "|", $second, "|", $rest["third"];
    }
}

$box = new CallBox();
$box->join(second: "S", first: "F", third: "R1");
