export interface Message {
  id: string;
  role: "user" | "pet";
  text: string;
}
