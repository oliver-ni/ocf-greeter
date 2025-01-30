{ lib, craneLib, autoPatchelfHook, pkg-config, glib, pango, cairo, gdk-pixbuf, gtk3, libsoup_3, webkitgtk_4_1, xdotool, wayland, libGL, libxkbcommon }:

craneLib.buildPackage {
  pname = "ocf-greeter";
  version = "2024-10-22";

  src = ./.;

  nativeBuildInputs = [ autoPatchelfHook pkg-config ];
  buildInputs = [ glib pango cairo gdk-pixbuf gtk3 libsoup_3 webkitgtk_4_1 xdotool ];
  runtimeDependencies = map lib.getLib [ wayland libGL libxkbcommon ];

  meta = with lib; {
    description = "Custom greetd greeter for the Open Computing Facility";
    homepage = "https://github.com/ocf/greeter";
    platforms = platforms.linux;
  };
}
