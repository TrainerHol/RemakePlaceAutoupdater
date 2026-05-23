declare module "lucide/dist/esm/icons/*.mjs" {
  type IconNode = Array<[string, Record<string, string | number>, IconNode?]>;
  const iconNode: IconNode;
  export default iconNode;
}
