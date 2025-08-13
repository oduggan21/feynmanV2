import { defineBuildConfig } from "obuild/config";

export default defineBuildConfig({
  entries: ["./src/handlers/handlers.ts", "./src/feynmanApi.schemas.ts"],
});
