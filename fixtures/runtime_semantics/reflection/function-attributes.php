<?php
// runtime-semantics: category=reflection expect=pass
#[FunctionMarker("function")]
function reflected_attribute_function(): void {
}

$attributes = (new ReflectionFunction("reflected_attribute_function"))->getAttributes();
echo $attributes[0]->getName(), ":", $attributes[0]->getArguments()[0], "\n";
