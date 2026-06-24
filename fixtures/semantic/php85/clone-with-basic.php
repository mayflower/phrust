<?php
class CloneWithBasicSubject
{
    public function __construct(public string $name = "old") {}
}

$copy = clone(new CloneWithBasicSubject(), ["name" => "new"]);
