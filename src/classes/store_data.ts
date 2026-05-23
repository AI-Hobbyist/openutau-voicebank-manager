export class StoreData {
  static saveSettings(key: string, data: Record<string, any>) {
    const jsonData = { vars: data };
    localStorage.setItem(key, JSON.stringify(jsonData));
  }

  static loadSettings(key: string): Record<string, any> | null {
    const data = localStorage.getItem(key);
    if (data) {
      const parsed = JSON.parse(data);
      return parsed.vars || null;
    }
    return null;
  }

  static saveConversations(key: string, conversations: Array<Record<string, any>>) {
    const jsonData = { convs: conversations };
    localStorage.setItem(key, JSON.stringify(jsonData));
  }

  static loadConversations(key: string): Array<Record<string, any>> | null {
    const data = localStorage.getItem(key);
    if (data) {
      const parsed = JSON.parse(data);
      return parsed.convs || [];
    }
    return [];
  }
}
