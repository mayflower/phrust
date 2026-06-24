<?php
// runtime-semantics: category=reflection expect=pass
#[ClassMarker("class", 7), ClassMarker("repeat")]
class AttributeClassTarget {
}

$attributes = (new ReflectionClass(AttributeClassTarget::class))->getAttributes();
echo $attributes[0]->getName(), ":", $attributes[0]->getArguments()[0], ":", $attributes[0]->getArguments()[1], "\n";
echo $attributes[1]->getName(), ":", $attributes[1]->getArguments()[0], "\n";
