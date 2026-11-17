import type { MeetingProfile } from './models';

export type ExampleConfigLanguage = 'zh-CN' | 'en';

export interface ExampleRosterSeedRow {
  key: string;
  canonicalName: string;
  pinyin: string;
  group: string;
  active: boolean;
}

export interface ExampleAliasSeedRow {
  zoomName: string;
  personKey: string;
  notes?: string;
}

export interface ExampleConfigBundle {
  profiles: MeetingProfile[];
  rosterPeople: ExampleRosterSeedRow[];
  aliasMappings: ExampleAliasSeedRow[];
}

export const EXAMPLE_PROFILE_EDITOR_PLACEHOLDERS: Record<
  ExampleConfigLanguage,
  {
    name: string;
    description: string;
    includedSubgroups: string;
    meetingIds: string;
    topicKeywords: string;
    groupSortOrder: string;
  }
> = {
  en: {
    name: 'e.g., Staff Meeting',
    description: 'e.g., Count only this group. No lateness rules.',
    includedSubgroups: 'e.g., Staff-* (wildcards supported)',
    meetingIds: 'e.g., 1234567890',
    topicKeywords: 'e.g., All Hands, Weekly',
    groupSortOrder: 'Staff-Admin\nStaff-Teachers\nStaff-Support',
  },
  'zh-CN': {
    name: '例：员工会议',
    description: '例：仅统计此分组成员，不启用迟到规则',
    includedSubgroups: '例：员工-*（支持通配符 *）',
    meetingIds: '例：1234567890',
    topicKeywords: '例：全体会议, 周会',
    groupSortOrder: '员工-行政\n员工-教师\n员工-后勤',
  },
};

const ENGLISH_BUNDLE: ExampleConfigBundle = {
  profiles: [
    {
      id: 'example-staff-meetings',
      name: 'Staff Meetings',
      description:
        'Meeting for staff members only, exclude attendance for students. No lateness rules.',
      includedSubgroups: ['Staff-*'],
      lateAfterMinutes: 99999,
      returnWindowMinutes: 99999,
      meetingIds: [],
      topicKeywords: ['Staff'],
      specialParticipants: [],
      blacklist: [],
      groupSortOrder: ['Staff-Admin', 'Staff-Teachers', 'Staff-Support'],
      active: true,
      isDefault: false,
    },
    {
      id: 'example-student-meeting',
      name: 'Student Meeting',
      description:
        'Meeting for students and teachers only, exclude attendance tracking for other groups. Lateness rules apply.',
      includedSubgroups: ['Staff-Teachers', 'Students-*'],
      lateAfterMinutes: 5,
      returnWindowMinutes: 5,
      meetingIds: [],
      topicKeywords: ['Student'],
      specialParticipants: [],
      blacklist: [],
      groupSortOrder: [
        'Students-HighSchool',
        'Students-MiddleSchool',
        'Students-ElementarySchool',
      ],
      active: true,
      isDefault: false,
    },
    {
      id: 'example-all-personnel',
      name: 'All Personnel Meeting',
      description: 'Meeting for everybody. Lateness rules apply.',
      includedSubgroups: ['*'],
      lateAfterMinutes: 5,
      returnWindowMinutes: 5,
      meetingIds: [],
      topicKeywords: ['All Hands', 'Town Hall'],
      specialParticipants: [],
      blacklist: [],
      groupSortOrder: ['Staff-*', 'Students-*'],
      active: true,
      isDefault: false,
    },
    {
      id: 'example-personal-meeting',
      name: 'My Personal Meeting',
      description:
        'Meeting ID fields override topic keywords during profile selection. This profile uses a fake Zoom Meeting ID example and is not the same as the app session ID.',
      includedSubgroups: ['*'],
      lateAfterMinutes: 5,
      returnWindowMinutes: 5,
      meetingIds: ['7492058316'],
      topicKeywords: [],
      specialParticipants: [],
      blacklist: [],
      groupSortOrder: [],
      active: true,
      isDefault: false,
    },
  ],
  rosterPeople: [
    {
      key: 'emma-reed',
      canonicalName: 'Emma Reed',
      pinyin: 'emmareed',
      group: 'Staff-Admin',
      active: true,
    },
    {
      key: 'noah-brooks',
      canonicalName: 'Noah Brooks',
      pinyin: 'noahbrooks',
      group: 'Staff-Admin',
      active: true,
    },
    {
      key: 'olivia-carter',
      canonicalName: 'Olivia Carter',
      pinyin: 'oliviacarter',
      group: 'Staff-Teachers',
      active: true,
    },
    {
      key: 'liam-bennett',
      canonicalName: 'Liam Bennett',
      pinyin: 'liambennett',
      group: 'Staff-Teachers',
      active: true,
    },
    {
      key: 'daniel-cruz',
      canonicalName: 'Daniel Cruz',
      pinyin: 'danielcruz',
      group: 'Staff-Support',
      active: true,
    },
    {
      key: 'grace-morgan',
      canonicalName: 'Grace Morgan',
      pinyin: 'gracemorgan',
      group: 'Staff-Support',
      active: true,
    },
    {
      key: 'ava-johnson',
      canonicalName: 'Ava Johnson',
      pinyin: 'avajohnson',
      group: 'Students-HighSchool',
      active: true,
    },
    {
      key: 'ethan-miller',
      canonicalName: 'Ethan Miller',
      pinyin: 'ethanmiller',
      group: 'Students-HighSchool',
      active: true,
    },
    {
      key: 'lily-nguyen',
      canonicalName: 'Lily Nguyen',
      pinyin: 'lilynguyen',
      group: 'Students-MiddleSchool',
      active: true,
    },
    {
      key: 'mason-lee',
      canonicalName: 'Mason Lee',
      pinyin: 'masonlee',
      group: 'Students-MiddleSchool',
      active: true,
    },
    {
      key: 'chloe-davis',
      canonicalName: 'Chloe Davis',
      pinyin: 'chloedavis',
      group: 'Students-ElementarySchool',
      active: true,
    },
    {
      key: 'lucas-young',
      canonicalName: 'Lucas Young',
      pinyin: 'lucasyoung',
      group: 'Students-ElementarySchool',
      active: true,
    },
  ],
  aliasMappings: [
    { zoomName: 'Emma R. (Laptop)', personKey: 'emma-reed' },
    { zoomName: 'Noah B Phone', personKey: 'noah-brooks' },
    { zoomName: 'Mrs Carter Zoom', personKey: 'olivia-carter' },
    { zoomName: 'Liam B - Tablet', personKey: 'liam-bennett' },
    { zoomName: 'Daniel C Ops', personKey: 'daniel-cruz' },
    { zoomName: 'Grace M Support', personKey: 'grace-morgan' },
    { zoomName: 'Ava J Student', personKey: 'ava-johnson' },
    { zoomName: 'Ethan M Student', personKey: 'ethan-miller' },
    { zoomName: 'Lily N iPad', personKey: 'lily-nguyen' },
    { zoomName: 'Mason L Home', personKey: 'mason-lee' },
    { zoomName: 'Chloe D Family Device', personKey: 'chloe-davis' },
    { zoomName: 'Lucas Y Parent Phone', personKey: 'lucas-young' },
  ],
};

const CHINESE_BUNDLE: ExampleConfigBundle = {
  profiles: [
    {
      id: 'example-staff-meetings-zh',
      name: '员工会议',
      description: '仅统计员工出席，不统计学生。不启用严格迟到规则。',
      includedSubgroups: ['员工-*'],
      lateAfterMinutes: 15,
      returnWindowMinutes: 10,
      meetingIds: [],
      topicKeywords: ['员工'],
      specialParticipants: [],
      blacklist: [],
      groupSortOrder: ['员工-行政', '员工-教师', '员工-后勤'],
      active: true,
      isDefault: false,
    },
    {
      id: 'example-student-meeting-zh',
      name: '学生会议',
      description: '仅统计学生和教师，其他分组不纳入考勤。启用迟到规则。',
      includedSubgroups: ['员工-教师', '学生-*'],
      lateAfterMinutes: 5,
      returnWindowMinutes: 5,
      meetingIds: [],
      topicKeywords: ['学生'],
      specialParticipants: [],
      blacklist: [],
      groupSortOrder: ['学生-高中', '学生-初中', '学生-小学'],
      active: true,
      isDefault: false,
    },
    {
      id: 'example-all-personnel-zh',
      name: '全员会议',
      description: '统计所有人员。启用迟到规则。',
      includedSubgroups: ['*'],
      lateAfterMinutes: 5,
      returnWindowMinutes: 5,
      meetingIds: [],
      topicKeywords: ['全体', '全员', '大会'],
      specialParticipants: [],
      blacklist: [],
      groupSortOrder: ['员工-*', '学生-*'],
      active: true,
      isDefault: false,
    },
    {
      id: 'example-personal-meeting-zh',
      name: '我的个人会议',
      description:
        '会议 ID 匹配会优先于主题关键词。这里使用的是虚构 Zoom 会议号示例，不是 App 会话 ID。',
      includedSubgroups: ['*'],
      lateAfterMinutes: 5,
      returnWindowMinutes: 5,
      meetingIds: ['7492058316'],
      topicKeywords: [],
      specialParticipants: [],
      blacklist: [],
      groupSortOrder: [],
      active: true,
      isDefault: false,
    },
  ],
  rosterPeople: [
    {
      key: 'wang-xiaotong',
      canonicalName: '王晓彤',
      pinyin: 'wangxiaotong',
      group: '员工-行政',
      active: true,
    },
    {
      key: 'li-wenjing',
      canonicalName: '李文静',
      pinyin: 'liwenjing',
      group: '员工-行政',
      active: true,
    },
    {
      key: 'chen-yating',
      canonicalName: '陈雅婷',
      pinyin: 'chenyating',
      group: '员工-教师',
      active: true,
    },
    {
      key: 'liu-zhiyuan',
      canonicalName: '刘志远',
      pinyin: 'liuzhiyuan',
      group: '员工-教师',
      active: true,
    },
    {
      key: 'zhang-guoqiang',
      canonicalName: '张国强',
      pinyin: 'zhangguoqiang',
      group: '员工-后勤',
      active: true,
    },
    {
      key: 'zhao-meiling',
      canonicalName: '赵美玲',
      pinyin: 'zhaomeiling',
      group: '员工-后勤',
      active: true,
    },
    {
      key: 'wang-zihan',
      canonicalName: '王子涵',
      pinyin: 'wangzihan',
      group: '学生-高中',
      active: true,
    },
    {
      key: 'chen-yutong',
      canonicalName: '陈雨桐',
      pinyin: 'chenyutong',
      group: '学生-高中',
      active: true,
    },
    {
      key: 'li-haoran',
      canonicalName: '李浩然',
      pinyin: 'lihaoran',
      group: '学生-初中',
      active: true,
    },
    {
      key: 'zhang-xinyi',
      canonicalName: '张欣怡',
      pinyin: 'zhangxinyi',
      group: '学生-初中',
      active: true,
    },
    {
      key: 'liu-chenxi',
      canonicalName: '刘晨曦',
      pinyin: 'liuchenxi',
      group: '学生-小学',
      active: true,
    },
    {
      key: 'zhao-yinuo',
      canonicalName: '赵一诺',
      pinyin: 'zhaoyinuo',
      group: '学生-小学',
      active: true,
    },
  ],
  aliasMappings: [
    { zoomName: '王晓彤-电脑', personKey: 'wang-xiaotong' },
    { zoomName: '李文静-iPad', personKey: 'li-wenjing' },
    { zoomName: '陈雅婷老师', personKey: 'chen-yating' },
    { zoomName: '刘志远-平板', personKey: 'liu-zhiyuan' },
    { zoomName: '张国强-后勤', personKey: 'zhang-guoqiang' },
    { zoomName: '赵美玲-手机', personKey: 'zhao-meiling' },
    { zoomName: '王子涵-学生端', personKey: 'wang-zihan' },
    { zoomName: '陈雨桐-家里', personKey: 'chen-yutong' },
    { zoomName: '李浩然-初中部', personKey: 'li-haoran' },
    { zoomName: '张欣怡-平板', personKey: 'zhang-xinyi' },
    { zoomName: '刘晨曦-家长手机', personKey: 'liu-chenxi' },
    { zoomName: '赵一诺-客厅设备', personKey: 'zhao-yinuo' },
  ],
};

export function getExampleConfigBundle(
  language: ExampleConfigLanguage
): ExampleConfigBundle {
  return language === 'en' ? ENGLISH_BUNDLE : CHINESE_BUNDLE;
}

