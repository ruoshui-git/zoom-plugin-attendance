// zc_users.hpp - owned participant copies used to build zc_user arrays.
#pragma once

#include <string>
#include <vector>

#include "zc_util.hpp"

namespace zc {

struct OwnedUser {
    uint32_t user_id = 0;
    uint32_t flags = 0;
    int32_t audio_type = 0;
    std::string name;  // UTF-8
};

// Holds owned strings and a parallel C view; the view is valid while this
// object is alive and unmodified.
class UserBatch {
public:
    void Add(OwnedUser u) { owned_.push_back(std::move(u)); }
    const zc_user *Data() {
        view_.clear();
        view_.reserve(owned_.size());
        for (const auto &u : owned_) {
            zc_user c{};
            c.user_id = u.user_id;
            c.flags = u.flags;
            c.audio_type = u.audio_type;
            c.name = u.name.data();
            c.name_len = u.name.size();
            view_.push_back(c);
        }
        return view_.empty() ? nullptr : view_.data();
    }
    size_t Size() const { return owned_.size(); }

private:
    std::vector<OwnedUser> owned_;
    std::vector<zc_user> view_;
};

}  // namespace zc
