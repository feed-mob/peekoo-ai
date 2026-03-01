export interface Message {
  id: string;
  role: "user" | "pet" | "error";
  text: string;
}
