<?php
// runtime-semantics: category=enums expect=pass
trait LabelsEnumCases {
    public function label(): string {
        return $this->name . "!";
    }
}

enum Labelled {
    use LabelsEnumCases;

    case Ready;
}

echo Labelled::Ready->label(), "\n";
