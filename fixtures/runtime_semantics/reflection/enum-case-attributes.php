<?php
// runtime-semantics: category=reflection expect=pass
enum AttributeEnumTarget {
    #[EnumCaseMarker("case")]
    case Ready;
}

$case = new ReflectionEnumUnitCase(AttributeEnumTarget::class, "Ready");
$attributes = $case->getAttributes();
echo $attributes[0]->getName(), ":", $attributes[0]->getArguments()[0], "\n";
