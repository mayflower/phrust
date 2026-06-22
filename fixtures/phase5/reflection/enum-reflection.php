<?php
// phase5-runtime: category=reflection expect=pass
#[EnumMarker("enum")]
enum Prompt28Status: string {
    #[EnumCaseMarker("case")]
    case Ready = "ready";
    case Done = "done";
}

$enum = new ReflectionEnum(Prompt28Status::class);
$cases = $enum->getCases();
$attributes = $enum->getAttributes();

echo $enum->getName(), "\n";
echo $enum->isBacked() ? "backed" : "unit", "\n";
echo $enum->getBackingType()->getName(), "\n";
echo $attributes[0]->getName(), ":", $attributes[0]->getArguments()[0], "\n";
echo $cases[0]->getName(), ":", $cases[0]->getBackingValue(), "\n";
echo $cases[0]->getAttributes()[0]->getName(), ":", $cases[0]->getAttributes()[0]->getArguments()[0], "\n";
