<?php

namespace Exprs;

class CloneSubject {
    public function __construct(public string $foo = "foo") {}
}

$subject = new CloneSubject();
$length = " value " |> trim(...) |> strlen(...);
$copy = clone($subject, ["foo" => "updated"]);
$plain = clone $subject;
$callable = strlen(...);
