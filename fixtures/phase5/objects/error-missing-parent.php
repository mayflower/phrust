<?php
// phase5-runtime: expect=fail
class MissingParentChild extends MissingParentBase {
}

new MissingParentChild();
