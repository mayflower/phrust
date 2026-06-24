<?php
// runtime-semantics: expect=fail
class MissingParentChild extends MissingParentBase {
}

new MissingParentChild();
