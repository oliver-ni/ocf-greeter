{ lib, craneLib, autoPatchelfHook, gcc-unwrapped, wayland, libGL, libxkbcommon }:

craneLib.buildPackage {
  pname = "ocf-greeter";
  version = "2024-10-22";

  src = ./.;

  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ gcc-unwrapped ];
  runtimeDependencies = map lib.getLib [ gcc-unwrapped wayland libGL libxkbcommon ];

  meta = with lib; {
    description = "Custom greetd greeter for the Open Computing Facility";
    homepage = "https://github.com/ocf/greeter";
    platforms = platforms.linux;
  };
}
