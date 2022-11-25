final: prev:
let
  inherit (prev.lib)
    zipAttrsWith concatLists isList
    head tail all isAttrs last unique
    recursiveUpdate;
in
{
  lib = recursiveUpdate (prev.lib or { }) {
    # Replicates // operator while additionally concatenating lists.
    recursiveMerge = attrList:
      let f = attrPath:
        zipAttrsWith (n: values:
          if tail values == [ ]
          then head values
          else if all isList values
          then unique (concatLists values)
          else if all isAttrs values
          then f (attrPath ++ [ n ]) values
          else last values
        );
      in f [ ] attrList;
  };
}
