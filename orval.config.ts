import type { Config } from "@orval/core";

export default {
  api: {
    output: {
      mode: "tags-split",
      target: "./packages/feynman-query/src",
      client: "react-query",
      mock: false,
      namingConvention: "snake_case",
      indexFiles: false,
    },
    input: {
      target: "./services/api/openapi.json",
    },
  },
} satisfies Config;
