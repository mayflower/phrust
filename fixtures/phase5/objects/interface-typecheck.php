<?php
interface AcceptsMe {}
class Impl implements AcceptsMe {}

function f(AcceptsMe $x): string {
    return ($x instanceof AcceptsMe) ? "accepted" : "rejected";
}

echo f(new Impl());
