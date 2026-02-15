module.exports = {
  plugins: ["prettier-plugin-svelte"],
  svelteSortOrder: "options-scripts-markup-styles",
  svelteStrictMode: true,
  svelteBracketNewLine: true,
  svelteAllowShorthand: true,
  overrides: [
    {
      files: "*.svelte",
      options: {
        parser: "svelte",
      },
    },
  ],
};
