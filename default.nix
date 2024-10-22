{ lib, rustPlatform, autoPatchelfHook, gcc-unwrapped, wayland, libGL, libxkbcommon }:

rustPlatform.buildRustPackage {
  pname = "ocf-greeter";
  version = "2024-10-08";

  src = ./.;

  cargoHash = "sha256-UkKSukPbKrXXrypSV5bdJiDU25qtocAbymIM3NaOIZw=";

  RUSTC_BOOTSTRAP = 1;

  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ gcc-unwrapped ];
  runtimeDependencies = map lib.getLib [ gcc-unwrapped wayland libGL libxkbcommon ];

  meta = with lib; {
    description = "Custom greetd greeter for the Open Computing Facility";
    homepage = "https://github.com/ocf/greeter";
    platforms = platforms.linux;
  };
}
