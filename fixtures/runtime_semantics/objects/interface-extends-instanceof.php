<?php
interface ParentFace {}
interface ChildFace extends ParentFace {}

class Thing implements ChildFace {}

$thing = new Thing();
echo (($thing instanceof Thing) ? "class" : "no");
echo "|";
echo (($thing instanceof ChildFace) ? "child" : "no");
echo "|";
echo (($thing instanceof ParentFace) ? "parent" : "no");
