<?php
function collect($first, $second = "B", ...$rest) {
    echo $first, "|", $second, "|", $rest["tail"];
}

collect(...["first" => "A", "tail" => "T"]);
