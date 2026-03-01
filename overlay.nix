final: prev: {
  multiclaw-web = final.callPackage ./web/package.nix { };

  multiclaw = final.callPackage ./package.nix {
    rustToolchain = final.fenix.stable.withComponents [
      "cargo"
      "clippy"
      "rust-src"
      "rustc"
      "rustfmt"
    ];
  };
}
